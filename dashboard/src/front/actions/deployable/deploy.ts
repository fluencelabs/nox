import {getNode, NodeId} from '../../../fluence';
import contract, {web3js} from '../../../fluence/contract';
import { Dispatch, Action } from 'redux';
import {DeployableApp} from "../../../fluence/deployable";
import {privateKey, defaultContractAddress, account} from "../../../constants";
let Tx = require('ethereumjs-tx');

export const DEPLOY_TX_SENT = 'DEPLOY_TX_SENT';

export const deploy = (app: DeployableApp) => {
    return async (dispatch: Dispatch): Promise<Action> => {
        let txData = contract.methods.addApp(app.storage_hash, "", app.cluster_size, []).encodeABI();
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
        let receipt = await web3js.eth.sendSignedTransaction(tx.serialize());
        if (receipt.events) {
            if (receipt.events.hasOwnProperty("AppEnqueued")) {
                console.log("enq " + receipt.events)
            } else if (receipt.events.hasOwnProperty("AppDeployed")) {
                console.log("depl " + receipt.events)
            }
        } else {
            console.log("no events " + receipt)
        }

        return dispatch({
            type: DEPLOY_TX_SENT
        })
    };
};

// export const retrieveNode = (nodeId: NodeId) => {
//     return async (dispatch: Dispatch): Promise<Action> => {
//         const node = await getNode(contract, nodeId);
//
//         return dispatch({
//             type: GET_NODE_RECEIVE,
//             node,
//         });
//     };
// };