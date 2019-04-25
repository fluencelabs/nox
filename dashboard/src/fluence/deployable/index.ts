import {TransactionReceipt} from "web3/types";
import {web3js} from "../contract";
import {account, defaultContractAddress} from "../../constants";
import {AppId} from "../apps";
import abi from '../../abi/Network.json';

import { parseLog } from "ethereum-event-logs"

export enum StorageType {
    Swarm = 0,
    Ipfs = 1
}

export type DeployableAppId = string;

export interface DeployableApp {
    name: string,
    shortName: string,
    storageHash: string,
    storageType: StorageType,
    clusterSize: number,
    selfUpload?: boolean,
}

export const deployableAppIds: DeployableAppId[] = ["redis", "llamadb", "upload"]; // 'dice, guess, tictactoe' are hidden intentionally

export const deployableApps: { [key: string]: DeployableApp } = {
    "llamadb": {
        name: "SQL DB (llamadb)",
        shortName: "llamadb",
        storageHash: "0x090A9B7CCA9D55A9632BBCC3A30A57F2DB1D1FD688659CFF95AB8D1F904AD74B",
        storageType: StorageType.Ipfs,
        clusterSize: 4
    },
    "dice": {
        // {"Name":"dice_game.wasm","Hash":"QmNsWcjoeMSjnVdBt4uGpwqEs24sQWjopP4wZyHsJ2CyMs","Size":"471184"}
        name: "Dice Game",
        shortName: "Dice Game",
        storageHash: "0x07E7DAD4A8C553BE9773C6E6FF9AECB70A46D84FE9B7EE379577AEE6A174C982",
        storageType: StorageType.Ipfs,
        clusterSize: 4
    },
    "guess": {
        // {"Name":"guessing_game.wasm","Hash":"QmPKt1idN3xDYC28sBbrCfQu3ZZShwvqGMTCAjmHodVAt5","Size":"242001"}
        name: "Guessing Game",
        shortName: "Guessing Game",
        storageHash: "0x0EA9260B083F8312DEDB4B37FFA40EA73E12E08E788A932C8D1B02B843A47936",
        storageType: StorageType.Ipfs,
        clusterSize: 4
    },
    "tictactoe": {
        // {"Name":"tic_tac_toe.wasm","Hash":"QmQw2qEJvCrgpH29PcNduzsTcmYYcGWcn9XRi6G5NroUri","Size":"508346"}
        name: "Tic Tac Toe",
        shortName: "Tic Tac Toe",
        storageHash: "0x268622BE3A3CB9473E764C229BE02ED9228170FF61F876CA3634590D748E1CEF",
        storageType: StorageType.Ipfs,
        clusterSize: 4
    },
    "upload": {
        name: "Upload your own app",
        shortName: "your app",
        storageHash: "",
        storageType: StorageType.Ipfs,
        clusterSize: 4,
        selfUpload: true
    },
    "redis": {
        name: "Redis (wasm-version 0.1)",
        shortName: "Redis",
        storageHash: "0xD5B85113DCB16A6457226F123A918AF4D90A47FB5FB89D7FC48247D3E1419D15",
        storageType: StorageType.Ipfs,
        clusterSize: 4,
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

export enum DeployedAppState {
    Enqueued,
    Deployed,
    Failed
}

export interface DeployedApp {
    state: DeployedAppState,
    appId: AppId | undefined
}

// Parse AppDeployed or AppEnqueued from TransactionReceipt
export function checkLogs(receipt: TransactionReceipt): DeployedApp {
    type AppEvent = { name: string, args: { appID: AppId } }
    let logs: AppEvent[] = parseLog(receipt.logs, abi);
    let enqueued = logs.find(l => l.name == "AppEnqueued");
    let deployed = logs.find(l => l.name == "AppDeployed");
    if (enqueued != undefined) {
        console.log("App enqueued with appID = " + enqueued.args.appID);
        return {
            state: DeployedAppState.Enqueued,
            appId: enqueued.args.appID
        };
    } else if (deployed != undefined) {
        console.log("App deployed with appID = " + deployed.args.appID);
        return {
            state: DeployedAppState.Deployed,
            appId: deployed.args.appID
        };
    }

    console.error("No AppDeployed or AppEnqueued event in logs: " + JSON.stringify(logs));
    return {
        state: DeployedAppState.Failed,
        appId: undefined
    };
}
