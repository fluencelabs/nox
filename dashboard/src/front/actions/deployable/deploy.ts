import { Action, Dispatch } from 'redux';
import { ThunkAction, ThunkDispatch } from 'redux-thunk';
import { History } from 'history';
import axios, { AxiosRequestConfig } from 'axios';
import EthereumTx from 'ethereumjs-tx';
import { getContract, getUserAddress, isMetamaskActive } from '../../../fluence/contract';
import {
    checkLogs,
    DeployableApp,
    send,
    txParams,
    DeployedAppState,
    sendUnsigned
} from '../../../fluence/deployable';
import { privateKey, appUploadUrl } from '../../../constants';
import { getApp, getNode, getNodeAppStatus } from '../../../fluence';
import { fromIpfsHash, storageToString32 } from '../../../utils';
import { retrieveAppRefs } from '..';
import { ReduxState } from '../../app';

export const DEPLOY_CLEAR_STATE = 'DEPLOY_CLEAR_STATE';
export const DEPLOY_STATE_PREPARE = 'DEPLOY_STATE_PREPARE';
export const DEPLOY_STATE_TRX = 'DEPLOY_STATE_TRX';
export const DEPLOY_STATE_CLUSTER_CHECK = 'DEPLOY_STATE_CLUSTER_CHECK';
export const DEPLOY_STATE_ENQUEUED = 'DEPLOY_STATE_ENQUEUED';
export const DEPLOY_TX_REVERTED = 'DEPLOY_TX_REVERTED';
export const APP_DEPLOYED = 'APP_DEPLOYED';
export const APP_ENQUEUED = 'APP_ENQUEUED';
export const APP_DEPLOY_FAILED = 'APP_DEPLOY_FAILED';

export interface DeployState {
    upload: {
        uploaded: boolean;
        uploading: boolean;
        data: any;
        storageHash: string;
        error?: any;
    };
    deployState?: {
        state: 'prepare'|'trx'|'enqueued'|'check_cluster';
        note?: string;
    };
}

const initialState: DeployState = {
    upload: {
        uploaded: false,
        uploading: false,
        data: {},
        storageHash: '',
    }
};

export const deploy = (
    app: DeployableApp,
    appTypeId: string,
    storageHashOverload: string,
    history: History
): ThunkAction<void, ReduxState, void, Action<string>> => {
    return async (dispatch: ThunkDispatch<any, void, Action>): Promise<Action> => {

        dispatch({type: DEPLOY_CLEAR_STATE});
        dispatch({type: DEPLOY_STATE_PREPARE});

        let storageHash = app.selfUpload && storageHashOverload ? storageHashOverload : app.storageHash;
        let storageType = storageToString32(app.storageType);
        let txData = getContract().methods.addApp(storageHash, '0x0', storageType, app.clusterSize, []).encodeABI();

        dispatch({type: DEPLOY_STATE_TRX});

        let receipt = null;
        if (isMetamaskActive()) {
                receipt = await sendUnsigned({
                    from: getUserAddress(),
                    ...await txParams(txData),
                });
        } else {
            let tx = new EthereumTx(await txParams(txData));
            tx.sign(privateKey);
            receipt = await send(tx.serialize());
        }

        if (!receipt.status) {
            return dispatch({type: DEPLOY_TX_REVERTED});
        }

        let deployStatus = checkLogs(receipt);

        if (deployStatus.state == DeployedAppState.Deployed) {
            dispatch({type: DEPLOY_STATE_CLUSTER_CHECK, note: 'retrieving app'});
            const deployedApp = await getApp(getContract(), String(deployStatus.appId));

            dispatch({type: DEPLOY_STATE_CLUSTER_CHECK, note: 'retrieving nodes'});
            const nodes = await Promise.all(
                deployedApp.cluster.get.node_ids.map(nodeId => getNode(getContract(), nodeId))
            );

            dispatch({type: DEPLOY_STATE_CLUSTER_CHECK, note: 'checking deploy status'});
            const checkCluster = async () => {
                const nodesAppStatus = await Promise.all(
                    nodes.map(node => getNodeAppStatus(node, String(deployStatus.appId)))
                );
                let nodesCount = nodesAppStatus.length;
                let readyNodes = 0;
                nodesAppStatus.forEach(nodesAppStatus => {
                    if (nodesAppStatus.status && Number(nodesAppStatus.status.sync_info.latest_block_height) >= 2) {
                        readyNodes++;
                    }
                });
                if (nodesCount == readyNodes) {
                    return true;
                }

                dispatch({type: DEPLOY_STATE_CLUSTER_CHECK, note: `checking deploy status, ${readyNodes} of ${nodesCount} are ready`});
                return new Promise((resolve, reject) => {
                    setTimeout(() => {
                        resolve(checkCluster());
                    }, 5000);
                });
            };

            await checkCluster();

            await dispatch(retrieveAppRefs());

            history.push(`/account/app/${deployStatus.appId}`);

            return dispatch({
                type: APP_DEPLOYED,
                appId: deployStatus.appId,
                app: app,
                trxHash: receipt.transactionHash
            });
        } else if (deployStatus.state == DeployedAppState.Enqueued) {
            return dispatch({type: DEPLOY_STATE_ENQUEUED});
        }

        return dispatch({
            type: APP_DEPLOY_FAILED,
            appId: deployStatus.appId,
            app: app,
            trxHash: receipt.transactionHash
        });
    };
};

export const DEPLOY_UPLOAD_STARTED = 'DEPLOY_UPLOAD_STARTED';
export const DEPLOY_UPLOAD_FINISHED = 'DEPLOY_UPLOAD_FINISHED';
export const DEPLOY_UPLOAD_FAILED = 'DEPLOY_UPLOAD_FAILED';
export const deployUpload = (form: FormData): ThunkAction<void, ReduxState, void, Action<string>> => {
    return async (dispatch: Dispatch): Promise<Action> => {

        dispatch({type: DEPLOY_UPLOAD_STARTED});

        const uploadParams: AxiosRequestConfig = {
            params: {
                pin: 'true',
            }
        };

        if (form.getAll('file').length > 1) {
            uploadParams.params['wrap-with-directory'] = 'true';
        }

        return axios.post(appUploadUrl, form, uploadParams).then(function (response) {
            let hash = '';
            if (typeof response.data === 'string') {
                const folderRaw = response.data.split('\n').filter(s => String(s).trim().length > 0).slice(-1)[0];
                const folder = JSON.parse(folderRaw);
                hash = folder.Hash;
            } else {
                hash = response.data.Hash;
            }

            return dispatch({
                type: DEPLOY_UPLOAD_FINISHED,
                result: response.data,
                storageHash: fromIpfsHash(hash)
            });
        }).catch(function (error) {
            return dispatch({
                type: DEPLOY_UPLOAD_FAILED,
                error: error
            });
        });
    };
};

export default ( state = initialState, action: any): DeployState => {
    switch (action.type) {
        case DEPLOY_CLEAR_STATE: {
            return initialState;
        }
        case DEPLOY_STATE_PREPARE: {
            return {
                ...state,
                deployState: {
                    state: 'prepare'
                }
            };
        }
        case DEPLOY_STATE_TRX: {
            return {
                ...state,
                deployState: {
                    state: 'trx'
                }
            };
        }
        case DEPLOY_STATE_ENQUEUED: {
            return {
                ...state,
                deployState: {
                    state: 'enqueued'
                }
            };
        }
        case DEPLOY_STATE_CLUSTER_CHECK: {
            return {
                ...state,
                deployState: {
                    state: 'check_cluster',
                    note: action.note
                }
            };
        }
        case APP_DEPLOYED:
        case APP_ENQUEUED: {
            return initialState;
        }
        case DEPLOY_UPLOAD_STARTED: {
            return {
                ...state,
                upload: {
                    ...state.upload,
                    uploading: true,
                }
            };
        }
        case DEPLOY_UPLOAD_FINISHED: {
            return {
                ...state,
                upload: {
                    ...state.upload,
                    uploading: false,
                    uploaded: true,
                    data: action.result,
                    storageHash: action.storageHash,
                }
            };
        }
        case DEPLOY_UPLOAD_FAILED: {
            return {
                ...state,
                upload: {
                    ...state.upload,
                    uploading: false,
                    uploaded: false,
                    error: action.error
                }
            };
        }
        default: {
            return state;
        }
    }
};
