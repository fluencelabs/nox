import {App, getAppIds, getApps} from '../../../fluence';
import contract, {web3js} from '../../../fluence/contract';
import {DeployableApp} from "../../../fluence/deployable";
import {account, defaultContractAddress, privateKey} from "../../../constants";
import {Action, Dispatch} from "redux";
import {TransactionReceipt} from "web3/types";

let EthereumTx = require("ethereumjs-tx");

export const DEPLOY_TX_SENT = 'DEPLOY_TX_SENT';
export const DEPLOY_TX_REVERTED = 'DEPLOY_TX_REVERTED';
export const APP_DEPLOYED = 'APP_DEPLOYED';
export const APP_ENQUEUED = 'APP_ENQUEUED';
export const APP_DEPLOY_TIMEOUT = 'APP_DEPLOY_TIMEOUT';

export const deploy = (app: DeployableApp) => {
    return async (dispatch: Dispatch): Promise<Action> => {
        let txData = contract.methods.addApp(app.storage_hash, "0x0", app.cluster_size, []).encodeABI();
        let tx = new EthereumTx(await txParams(txData));
        tx.sign(privateKey);

        let receipt = await send(tx.serialize(), dispatch);

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

function send(signedTx: Buffer, dispatch: Dispatch): Promise<TransactionReceipt> {
    return web3js
        .eth
        .sendSignedTransaction('0x' + signedTx.toString('hex'))
        .once("transactionHash", h => {
            dispatch({type: DEPLOY_TX_SENT});
            console.log("tx hash " + h)
        });
}

async function txParams(txData: string): Promise<any> {
    let nonce = web3js.utils.numberToHex(await web3js.eth.getTransactionCount(account, "pending"));
    let gasPrice = web3js.utils.numberToHex(await web3js.eth.getGasPrice());
    let gasLimit = web3js.utils.numberToHex(1000000);
    return {
        nonce: nonce,
        gasPrice: gasPrice,
        gasLimit: gasLimit,
        to: defaultContractAddress,
        value: '0x00',
        data: txData,
        // EIP 155 chainId - mainnet: 1, rinkeby: 4
        chainId: 4
    };
}

async function waitApp(app: DeployableApp): Promise<[string, App | undefined]> {
    let appeared = false;
    for (let i = 0; i < 10 && !appeared; i++) {
        console.log("checking status");
        let appStatus = await checkStatus(app);
        if (appStatus != undefined) {
            appeared = true;
            if (appStatus.cluster.isDefined) {
                console.log("App deployed " + JSON.stringify(appStatus));
                return [APP_DEPLOYED, appStatus];
            } else {
                console.log("App enqueued " + JSON.stringify(appStatus));
                return [APP_ENQUEUED, appStatus];
            }
        }
    }

    console.log("App deployment timed out :(");
    return [APP_DEPLOY_TIMEOUT, undefined];
}

async function checkStatus(deployableApp: DeployableApp) {
    let ids = await getAppIds(contract).catch(e => {
        console.log("error while getAppIds " + JSON.stringify(e));
        return [];
    });
    let apps = await getApps(contract, ids).catch(e => {
        console.log("error while getApps " + JSON.stringify(e));
        let res: App[] = [];
        return res;
    });
    return apps.find(a => a.storage_hash == deployableApp.storage_hash);
}

/*
 * Reducer
 */
export default (state = {}, action: any) => {
    return state;
};
