import { TransactionReceipt } from 'web3/types';
import { getWeb3Js } from '../contract';
import { account, defaultContractAddress } from '../../constants';
import { AppId } from '../apps';
import abi from '../../abi/Network.json';

import { parseLog } from 'ethereum-event-logs';
import {Tx} from "web3/eth/types";

export enum StorageType {
    Swarm = 0,
    Ipfs = 1
}

export type DeployableAppId = string;

export interface DeployableApp {
    name: string;
    shortName: string;
    storageHash: string;
    storageType: StorageType;
    clusterSize: number;
    repoLink?: string;
    selfUpload?: boolean;
    requestExamples?: string[];
}

export const deployableAppIds: DeployableAppId[] = ['sqlite', 'redis', 'llamadb', 'upload']; // 'dice, guess, tictactoe' are hidden intentionally

export const deployableApps: { [key: string]: DeployableApp } = {
    llamadb: {
        name: 'LlamaDB fork (sql, wasm v0.1.2)',
        shortName: 'LlamaDB fork',
        storageHash: '0x090A9B7CCA9D55A9632BBCC3A30A57F2DB1D1FD688659CFF95AB8D1F904AD74B',
        storageType: StorageType.Ipfs,
        clusterSize: 4,
        repoLink: 'https://github.com/nukep/llamadb/compare/master...fluencelabs:master',
        requestExamples: ['CREATE TABLE users(id int, name varchar(128), age int)',
                          'INSERT INTO users VALUES(1, \'Sara\', 23), (2, \'Bob\', 19), (3, \'Caroline\', 31), (4, \'Max\', 27)',
                          'SELECT AVG(age) FROM users']
    },
    dice: {
        // {"Name":"dice_game.wasm","Hash":"QmNsWcjoeMSjnVdBt4uGpwqEs24sQWjopP4wZyHsJ2CyMs","Size":"471184"}
        name: 'Dice Game',
        shortName: 'Dice Game',
        storageHash: '0x07E7DAD4A8C553BE9773C6E6FF9AECB70A46D84FE9B7EE379577AEE6A174C982',
        storageType: StorageType.Ipfs,
        clusterSize: 4
    },
    guess: {
        // {"Name":"guessing_game.wasm","Hash":"QmPKt1idN3xDYC28sBbrCfQu3ZZShwvqGMTCAjmHodVAt5","Size":"242001"}
        name: 'Guessing Game',
        shortName: 'Guessing Game',
        storageHash: '0x0EA9260B083F8312DEDB4B37FFA40EA73E12E08E788A932C8D1B02B843A47936',
        storageType: StorageType.Ipfs,
        clusterSize: 4
    },
    tictactoe: {
        // {"Name":"tic_tac_toe.wasm","Hash":"QmQw2qEJvCrgpH29PcNduzsTcmYYcGWcn9XRi6G5NroUri","Size":"508346"}
        name: 'Tic Tac Toe',
        shortName: 'Tic Tac Toe',
        storageHash: '0x268622BE3A3CB9473E764C229BE02ED9228170FF61F876CA3634590D748E1CEF',
        storageType: StorageType.Ipfs,
        clusterSize: 4
    },
    upload: {
        name: 'Upload your own app',
        shortName: 'your app',
        storageHash: '',
        storageType: StorageType.Ipfs,
        clusterSize: 4,
        selfUpload: true
    },
    // {"Name":"redis_0.5.wasm","Hash":"QmYmpLNRaWEat3pUXxohYyceDCnLAvQGDZrM6hA26VxUbf","Size":"597690"}
    redis: {
        name: 'Redis fork (nosql, wasm v0.4)',
        shortName: 'Redis fork',
        storageHash: '0x9B0745D4B2D9292D6986453C79EA75E7385AB0AF128E0E65DBB7C478218709DE',
        storageType: StorageType.Ipfs,
        clusterSize: 4,
        repoLink: 'https://github.com/fluencelabs/redis/compare/5.0...fluencelabs:wasm',
        requestExamples: ['SET A 10',
                          'SADD B 20',
                          'GET A',
                          'SMEMBERS B',
                          `eval "redis.call('incr', 'A') return redis.call('get', 'A') * 8 + 5"  0`]
    },
    // {"Name":"sqlite3_0.1.wasm","Hash":"QmcnVd4qfcy3QA6RfCJdxtiMxafYjm6yTLAdk7N5qHCQA6","Size":"965949"}
    sqlite: {
        name: 'SQLite fork (sql, wasm v0.1)',
        shortName: 'SQLite fork',
        storageHash: '0xD6A2718452E0BE637826D97702953FC6F4F2226B23E724920926F98783A41083',
        storageType: StorageType.Ipfs,
        clusterSize: 4,
        repoLink: 'https://github.com/fluencelabs/sqlite/compare/original...master',
        requestExamples: ['CREATE VIRTUAL TABLE posts USING FTS5(body)',
                          'INSERT INTO posts(body) VALUES(\'AB\'), (\'BC\'), (\'CD\'), (\'DE\')',
                          'SELECT * FROM posts WHERE posts MATCH \'A* OR B*\'']
    }
};

// Sends a signed transaction to Ethereum
export function send(signedTx: Buffer): Promise<TransactionReceipt> {
    return getWeb3Js()
        .eth
        .sendSignedTransaction('0x' + signedTx.toString('hex'))
        .once('transactionHash', (h: string) => {
            console.log('tx hash ' + h);
        });
}

// Sends a transaction to Ethereum
export function sendUnsigned(tx: Tx): Promise<TransactionReceipt> {
    return getWeb3Js()
        .eth
        .sendTransaction(tx)
        .once('transactionHash', (h: string) => {
            console.log('tx hash ' + h);
        });
}

// Builds TxParams object to later use for building a transaction
export async function txParams(txData: string): Promise<any> {
    const nonce = getWeb3Js().utils.numberToHex(await getWeb3Js().eth.getTransactionCount(account, 'pending'));
    const gasPrice = getWeb3Js().utils.numberToHex(await getWeb3Js().eth.getGasPrice());
    const gasLimit = getWeb3Js().utils.numberToHex(1000000);

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
    state: DeployedAppState;
    appId: AppId | undefined;
}

// Parse AppDeployed or AppEnqueued from TransactionReceipt
export function checkLogs(receipt: TransactionReceipt): DeployedApp {
    type AppEvent = { name: string; args: { appID: AppId } };
    const logs: AppEvent[] = parseLog(receipt.logs, abi);
    const enqueued = logs.find(l => l.name === 'AppEnqueued');
    const deployed = logs.find(l => l.name === 'AppDeployed');
    if (enqueued !== undefined) {
        console.log('App enqueued with appID = ' + enqueued.args.appID);

        return {
            state: DeployedAppState.Enqueued,
            appId: enqueued.args.appID
        };
    } else if (deployed != undefined) {
        console.log('App deployed with appID = ' + deployed.args.appID);

        return {
            state: DeployedAppState.Deployed,
            appId: deployed.args.appID
        };
    }

    console.error('No AppDeployed or AppEnqueued event in logs: ' + JSON.stringify(logs));

    return {
        state: DeployedAppState.Failed,
        appId: undefined
    };
}

export function findDeployableAppByStorageHash(storageHash: string): DeployableApp | undefined {
    const deployableApp = Object.values(deployableApps).find(deployableApp => {
        return deployableApp.storageHash.toLowerCase() == storageHash.toLowerCase();
    });

    return deployableApp ? deployableApp : undefined;
}
