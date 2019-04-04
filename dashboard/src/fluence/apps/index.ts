import {Network} from "../../../types/web3-contracts/Network";
import {none, Option, some} from "ts-option";

export type AppId = string;

/**
 * An app is a WASM file. It can be deployed on a real-time cluster and run.
 * In Fluence contract it represents as a Swarm address to the WASM file
 * and a requirement of how many nodes will be in the cluster.
 */
export interface App {
    app_id: string,
    storage_hash: string,
    storage_receipt: string,
    storage_type: number,
    cluster_size: number,
    owner: string,
    pinToNodes: string[],
    cluster: Option<Cluster>
}


export interface Cluster {
    genesis_time: number,
    node_ids: string[]
}

export async function getAppIds(contract: Network): Promise<AppId[]> {
    return contract.methods.getAppIDs().call();
}

export function getApp(contract: Network, id: AppId): Promise<App> {
    return contract.methods.getApp(id).call().then((unparsedApp) => {
        let storageHash: string = unparsedApp["0"];
        let storageReceipt: string = unparsedApp["1"];
        let storageType: number = parseInt(unparsedApp["2"]);
        let clusterSize: number = parseInt(unparsedApp["3"]);
        let owner: string = unparsedApp["4"];
        let pinToNodes: string[] = unparsedApp["5"];

        let genesisTime: number = parseInt(unparsedApp["6"]);
        let nodeIds: string[] = unparsedApp["7"];

        let clusterOpt = parseCluster(genesisTime, nodeIds);

        return {
            app_id: id,
            storage_hash: storageHash,
            storage_receipt: storageReceipt,
            storage_type: storageType,
            cluster_size: clusterSize,
            owner: owner,
            pinToNodes: pinToNodes,
            cluster: clusterOpt
        };
    });
}

/**
 * Gets list of enqueued codes from Fluence contract
 */
export async function getApps(contract: Network, ids: AppId[]): Promise<App[]> {

    let appCalls: Promise<App>[] = ids.map((id) => {
        return getApp(contract, id)
    });

    return Promise.all(appCalls);
}

export function parseCluster(genesisTime: number, nodeIds: string[]): Option<Cluster> {
    if (genesisTime !== 0) {
        return some({
            genesis_time: genesisTime,
            node_ids: nodeIds
        });
    } else { return none }
}
