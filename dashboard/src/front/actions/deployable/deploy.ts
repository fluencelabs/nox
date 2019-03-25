import contract from '../../../fluence/contract';
import {checkLogs, DeployableApp, send, txParams, deployableApps} from "../../../fluence/deployable";
import {privateKey} from "../../../constants";
import {Action, Dispatch} from "redux";
import Cookies from 'js-cookie';
import EthereumTx from "ethereumjs-tx";

export const DEPLOY_STATE_PREPARE = 'DEPLOY_STATE_PREPARE';
export const DEPLOY_STATE_TRX = 'DEPLOY_STATE_TRX';
export const DEPLOY_TX_REVERTED = 'DEPLOY_TX_REVERTED';
export const APP_DEPLOYED = 'APP_DEPLOYED';
export const APP_ENQUEUED = 'APP_ENQUEUED';
export const APP_DEPLOY_FAILED = 'APP_DEPLOY_FAILED';

export const deploy = (app: DeployableApp, appTypeId: string) => {
    return async (dispatch: Dispatch): Promise<Action> => {

        dispatch({type: DEPLOY_STATE_PREPARE});

        let txData = contract.methods.addApp(app.storageHash, "0x0", app.clusterSize, []).encodeABI();
        let tx = new EthereumTx(await txParams(txData));
        tx.sign(privateKey);

        dispatch({type: DEPLOY_STATE_TRX});
        let receipt = await send(tx.serialize());

        if (!receipt.status) {
            return dispatch({type: DEPLOY_TX_REVERTED});
        }

        let [type, appId] = checkLogs(receipt);

        Cookies.remove('deployedAppId');
        Cookies.remove('deployedAppTypeId');
        Cookies.set('deployedAppId', String(appId), { expires: 365 });
        Cookies.set('deployedAppTypeId', String(appTypeId), { expires: 365 });

        return dispatch({
            type: type,
            appId: appId,
            app: app,
            trxHash: receipt.transactionHash
        });
    };
};

export const DEPLOY_RESTORE = 'DEPLOY_RESTORE';
export const restoreDeployed = (appId: string, appTypeId: string) => {
    return (dispatch: Dispatch): Action => {
        return dispatch({
            type: DEPLOY_RESTORE,
            appId,
            app: deployableApps[appTypeId],
        });
    };
};

export default (state = {}, action: any) => {
    switch (action.type) {
        case DEPLOY_STATE_PREPARE: {
            return {
                ...state,
                deployState: 'prepare'
            };
        }
        case DEPLOY_STATE_TRX: {
            return {
                ...state,
                deployState: 'trx'
            };
        }
        case APP_DEPLOYED:
        case APP_ENQUEUED: {
            return {
                ...state,
                deployState: 'end',
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
        default: {
            return state;
        }
    }
};