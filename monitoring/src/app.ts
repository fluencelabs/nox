/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

import {Network} from "../types/web3-contracts/Network";
import {Option} from "ts-option";
import {Cluster, parseCluster} from "./cluster";

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

/**
 * Gets list of enqueued codes from Fluence contract
 */
export async function getApps(contract: Network, ids: string[]): Promise<App[]> {

    let appCalls: Promise<App>[] = ids.map((id) => {
        return contract.methods.getApp(id).call().then((unparsedApp) => {
            let storageHash: string = unparsedApp["0"];
            let storageReceipt: string = unparsedApp["1"];
            let clusterSize: number = parseInt(unparsedApp["2"]);
            let owner: string = unparsedApp["3"];
            let pinToNodes: string[] = unparsedApp["4"];

            let genesisTime: number = parseInt(unparsedApp["5"]);
            let nodeIds: string[] = unparsedApp["6"];
            let ports: number[] = unparsedApp["7"].map(parseInt);

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
    });

    return Promise.all(appCalls);
}
