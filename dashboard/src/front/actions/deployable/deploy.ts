import {getAppIds, getApps} from '../../../fluence';
import contract, {web3js} from '../../../fluence/contract';
import {DeployableApp} from "../../../fluence/deployable";
import {account, defaultContractAddress, privateKey} from "../../../constants";

let Tx = require('ethereumjs-tx');

export const DEPLOY_TX_SENT = 'DEPLOY_TX_SENT';

export const deploy = async (app: DeployableApp) => {
    // return async (dispatch: Dispatch): Promise<Action> => {
    console.log("started deploy");
    let txData = contract.methods.addApp(app.storage_hash, "0x0", app.cluster_size, []).encodeABI();
    let nonce = web3js.utils.numberToHex(await web3js.eth.getTransactionCount(account, "pending"));
    let gasPrice = web3js.utils.numberToHex(await web3js.eth.getGasPrice());
    let gasLimit = web3js.utils.numberToHex(1000000);
    const txParams = {
        nonce: nonce,
        gasPrice: gasPrice,
        gasLimit: gasLimit,
        to: defaultContractAddress,
        value: '0x00',
        data: txData,
        // EIP 155 chainId - mainnet: 1, rinkeby: 4
        chainId: 4
    };
    let tx = new Tx(txParams);
    tx.sign(privateKey);
    let signed = tx.serialize();
    console.log("signed");

    let receipt = await web3js
        .eth
        .sendSignedTransaction('0x' + signed.toString('hex'))
        .once("transactionHash", h => {
            console.log("tx hash " + h)
        });
    console.log("sent");

    if (receipt.events) {
        if (receipt.events.hasOwnProperty("AppEnqueued")) {
            console.log("enq " + JSON.stringify(receipt.events))
        } else if (receipt.events.hasOwnProperty("AppDeployed")) {
            console.log("depl " + JSON.stringify(receipt.events))
        }
    } else {
        // there could be no logs because we're using Ethereum light node
        console.log("no events " + JSON.stringify(receipt));

        let appeared = false;
        for (let i = 0; i < 10 && !appeared; i++) {
            console.log("checking status");
            let appStatus = await checkStatus(app);
            if (appStatus != undefined) {
                appeared = true;
                if (appStatus.cluster.isDefined) {
                    console.log("deployed");
                } else {
                    console.log("enqueued");
                }
            }
        }
    }

    // return dispatch({
    //     type: DEPLOY_TX_SENT
    // })
    // };
};

export const checkStatus = async (deployableApp: DeployableApp) => {
    let ids = await getAppIds(contract);
    let apps = await getApps(contract, ids);
    return apps.find(a => a.storage_hash == deployableApp.storage_hash);
};

/*
 * Reducer
 */
export default (state = {}, action: any) => {
    switch (action.type) {
        case DEPLOY_TX_SENT: {
            console.log("Reducer: DEPLOY_TX_SENT. Wubba lubba dub dub.");
            return state;
        }
        default: {
            return state;
        }
    }
};
