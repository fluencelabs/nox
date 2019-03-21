import contract from '../../../fluence/contract';
import {DeployableApp, send, txParams, waitApp} from "../../../fluence/deployable";
import {privateKey} from "../../../constants";
import {Action, Dispatch} from "redux";

let EthereumTx = require("ethereumjs-tx");

export const DEPLOY_TX_REVERTED = 'DEPLOY_TX_REVERTED';
export const APP_DEPLOYED = 'APP_DEPLOYED';
export const APP_ENQUEUED = 'APP_ENQUEUED';
export const APP_DEPLOY_TIMEOUT = 'APP_DEPLOY_TIMEOUT';

export const deploy = (app: DeployableApp) => {
    return async (dispatch: Dispatch): Promise<Action> => {
        let txData = contract.methods.addApp(app.storageHash, "0x0", app.clusterSize, []).encodeABI();
        let tx = new EthereumTx(await txParams(txData));
        tx.sign(privateKey);

        let receipt = await send(tx.serialize());

        if (!receipt.status) {
            return dispatch({type: DEPLOY_TX_REVERTED});
        }

        let [type, appStatus] = await waitApp(app);

        return dispatch({
            type: type,
            appStatus: appStatus,
            app: app
        });
    };
};

