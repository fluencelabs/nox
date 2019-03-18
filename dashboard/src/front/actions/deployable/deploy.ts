import {getNode, NodeId} from '../../../fluence';
import contract, {web3js} from '../../../fluence/contract';
import {Dispatch, Action} from 'redux';
import {DeployableApp} from "../../../fluence/deployable";
import {privateKey, defaultContractAddress, account} from "../../../constants";
import {GET_NODE_RECEIVE} from "../nodes/nodes";

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
    let receipt = await web3js.eth.sendSignedTransaction('0x' + signed.toString('hex'))
        .once("transactionHash", h => console.log("got tx hash " + h));
    console.log("sent");
    if (receipt.events) {
        if (receipt.events.hasOwnProperty("AppEnqueued")) {
            console.log("enq " + receipt.events)
        } else if (receipt.events.hasOwnProperty("AppDeployed")) {
            console.log("depl " + receipt.events)
        }
    } else {
        console.log("no events " + receipt)
    }

    // return dispatch({
    //     type: DEPLOY_TX_SENT
    // })
    // };
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
