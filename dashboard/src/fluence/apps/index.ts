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
    cluster_size: number,
    owner: string,
    pinToNodes: string[],
    cluster: Option<Cluster>
}

export interface ClusterMember {
    id: string,
    port: number
}

export interface Cluster {
    genesis_time: number,
    cluster_members: ClusterMember[]
}

export async function getAppIds(contract: Network): Promise<AppId[]> {
    return contract.methods.getAppIDs().call();
}

export function getApp(contract: Network, id: AppId): Promise<App> {
    return contract.methods.getApp(id).call().then((unparsedApp) => {
        let storageHash: string = unparsedApp["0"];
        let storageReceipt: string = unparsedApp["1"];
        let clusterSize: number = parseInt(unparsedApp["2"]);
        let owner: string = unparsedApp["3"];
        let pinToNodes: string[] = unparsedApp["4"];

        let genesisTime: number = parseInt(unparsedApp["5"]);
        let nodeIds: string[] = unparsedApp["6"];
        let ports: number[] = unparsedApp["7"].map((p) => parseInt(p));

        let clusterOpt = parseCluster(genesisTime, nodeIds, ports);

        return {
            app_id: id,
            storage_hash: storageHash,
            storage_receipt: storageReceipt,
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

export function parseCluster(genesisTime: number, nodeIds: string[], ports: number[]): Option<Cluster> {
    if (genesisTime !== 0) {
        let clusterMembers: ClusterMember[] = ports.map((port, idx) => {
            return {
                id: nodeIds[idx],
                port: port
            }
        });

        return some({
            genesis_time: genesisTime,
            cluster_members: clusterMembers
        });
    } else { return none }
}