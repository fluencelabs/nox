import {TransactionReceipt} from "web3/types";
import contract, {web3js} from "../contract";
import {account, defaultContractAddress} from "../../constants";
import {App, getAppIds, getApps} from "../apps";
import {APP_DEPLOY_TIMEOUT, APP_DEPLOYED, APP_ENQUEUED} from "../../front/actions/deployable/deploy";

export type DeployableAppId = string;

export interface DeployableApp {
    name: string,
    storage_hash: string,
    cluster_size: number,
}

export const deployableAppIds: [DeployableAppId] = ["llamadb"];

export const deployableApps: { [key: string]: DeployableApp } = {
    "llamadb": {
        name: "SQL DB (llamadb)",
        storage_hash: "0x9918b8657755b41096da7a7da0528550ffce4a812c2295d2811c86d29be23326",
        cluster_size: 4
    }
};

// Sends a signed transaction to Ethereum
export function send(signedTx: Buffer): Promise<TransactionReceipt> {
    return web3js
        .eth
        .sendSignedTransaction('0x' + signedTx.toString('hex'))
        .once("transactionHash", h => {
            console.log("tx hash " + h)
        });
}

// Builds TxParams object to later use for building a transaction
export async function txParams(txData: string): Promise<any> {
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

// Waits until app is accepted by a smart-contract
export async function waitApp(app: DeployableApp): Promise<[string, App | undefined]> {
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

// Loads app from the smart contract's status
export async function checkStatus(deployableApp: DeployableApp): Promise<App | undefined> {
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