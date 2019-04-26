import contract from '../../../fluence/contract';
import {checkLogs, DeployableApp, send, txParams, deployableApps, DeployedAppState} from "../../../fluence/deployable";
import {History} from 'history';
import {privateKey, appUploadUrl} from "../../../constants";
import {Action, Dispatch} from "redux";
import axios from 'axios';
import EthereumTx from "ethereumjs-tx";
import {getApp, getNode, getNodeAppStatus} from "../../../fluence";
import {fromIpfsHash, storageToString32} from "../../../utils";
import {clearDeployedApp, saveDeployedApp} from "../../../utils/cookie";

export const DEPLOY_CLEAR_STATE = 'DEPLOY_CLEAR_STATE';
export const DEPLOY_STATE_PREPARE = 'DEPLOY_STATE_PREPARE';
export const DEPLOY_STATE_TRX = 'DEPLOY_STATE_TRX';
export const DEPLOY_STATE_CLUSTER_CHECK = 'DEPLOY_STATE_CLUSTER_CHECK';
export const DEPLOY_STATE_ENQUEUED = 'DEPLOY_STATE_ENQUEUED';
export const DEPLOY_TX_REVERTED = 'DEPLOY_TX_REVERTED';
export const APP_DEPLOYED = 'APP_DEPLOYED';
export const APP_ENQUEUED = 'APP_ENQUEUED';
export const APP_DEPLOY_FAILED = 'APP_DEPLOY_FAILED';

export const deploy = (app: DeployableApp, appTypeId: string, storageHashOverload: string, history: History) => {
    return async (dispatch: Dispatch): Promise<Action> => {

        dispatch({type: DEPLOY_CLEAR_STATE});
        dispatch({type: DEPLOY_STATE_PREPARE});

        let storageHash = app.selfUpload && storageHashOverload ? storageHashOverload : app.storageHash;
        let storageType = storageToString32(app.storageType);
        let txData = contract.methods.addApp(storageHash, "0x0", storageType, app.clusterSize, []).encodeABI();
        let tx = new EthereumTx(await txParams(txData));
        tx.sign(privateKey);

        dispatch({type: DEPLOY_STATE_TRX});
        let receipt = await send(tx.serialize());

        if (!receipt.status) {
            return dispatch({type: DEPLOY_TX_REVERTED});
        }

        let deployStatus = checkLogs(receipt);

        saveDeployedApp(String(deployStatus.appId), appTypeId);
        history.push(`/deploy/${appTypeId}/${deployStatus.appId}`);

        if (deployStatus.state == DeployedAppState.Deployed) {
            dispatch({type: DEPLOY_STATE_CLUSTER_CHECK, note: 'retrieving app'});
            const deployedApp = await getApp(contract, String(deployStatus.appId));

            dispatch({type: DEPLOY_STATE_CLUSTER_CHECK, note: 'retrieving nodes'});
            const nodes = await Promise.all(
                deployedApp.cluster.get.node_ids.map(nodeId => getNode(contract, nodeId))
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

export const DEPLOY_RESTORE = 'DEPLOY_RESTORE';
export const restoreDeployed = (appId: string, appTypeId: string, history: History) => {
    return (dispatch: Dispatch): Action => {
        history.push(`/deploy/${appTypeId}/${appId}`);
        return dispatch({
            type: DEPLOY_RESTORE,
            appId,
            app: deployableApps[appTypeId],
        });
    };
};

export const DEPLOY_RESET = 'DEPLOY_RESET';
export const resetDeployed = () => {
    return (dispatch: Dispatch): Action => {
        clearDeployedApp();
        return dispatch({
            type: DEPLOY_RESET
        });
    };
};

export const DEPLOY_UPLOAD_STARTED = 'DEPLOY_UPLOAD_STARTED';
export const DEPLOY_UPLOAD_FINISHED = 'DEPLOY_UPLOAD_FINISHED';
export const DEPLOY_UPLOAD_FAILED = 'DEPLOY_UPLOAD_FAILED';
export const deployUpload = (form: FormData) => {
    return async (dispatch: Dispatch): Promise<Action> => {

        dispatch({type: DEPLOY_UPLOAD_STARTED});

        return axios.post(appUploadUrl, form).then(function (response) {
            return dispatch({
                type: DEPLOY_UPLOAD_FINISHED,
                result: response.data,
                storageHash: fromIpfsHash(response.data.Hash)
            });
        }).catch(function (error) {
            return dispatch({
                type: DEPLOY_UPLOAD_FAILED,
                error: error
            });
        });
    };
};

export default (state = {
    upload: {
        uploaded: false,
        uploading: false,
        data: {},
        storageHash: '',
    }
}, action: any) => {
    switch (action.type) {
        case DEPLOY_CLEAR_STATE: {
            return {
                ...state,
                app: undefined,
                appId: undefined,
                trxHash: undefined
            };
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
            return {
                ...state,
                deployState: {
                    state: 'end'
                },
                app: action.app,
                appId: action.appId,
                trxHash: action.trxHash
            };
        }
        case DEPLOY_RESTORE: {
            return {
                ...state,
                deployState: 'end',
                app: action.app,
                appId: action.appId,
            };
        }
        case DEPLOY_RESET: {
            return {
                ...state,
                deployState: undefined,
                app: undefined,
                appId: undefined,
            };
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
