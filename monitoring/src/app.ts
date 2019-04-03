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
    storage_type: number,
    storage_receipt: string,
    cluster_size: number,
    owner: string,
    pinToNodes: string[],
    cluster: Option<Cluster>
}

export function getApp(contract: Network, id: string): Promise<App> {
    return contract.methods.getApp(id).call()
        .catch((e) => {
            console.error(`Error ocurred on \`getApp\` method. Check if contract address ${contract._address} or id ${id} is correct. Cause: ${e}`);
            throw e;
        }).then((unparsedApp) => {
            let storageHash: string = unparsedApp["0"];
            let storageReceipt: string = unparsedApp["1"];
            let storageType: number = parseInt(unparsedApp["2"]);
            let clusterSize: number = parseInt(unparsedApp["3"]);
            let owner: string = unparsedApp["4"];
            let pinToNodes: string[] = unparsedApp["5"];

            let genesisTime: number = parseInt(unparsedApp["6"]);
            let nodeIds: string[] = unparsedApp["7"];

            let clusterOpt;
            try {
                 clusterOpt = parseCluster(genesisTime, nodeIds);
            } catch (e) {
                console.error("Error occured on cluster parsing.");
                throw e;
            }

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
export async function getApps(contract: Network, ids: string[]): Promise<App[]> {

    let appCalls: Promise<App>[] = ids.map((id) => {
        return getApp(contract, id)
    });

    return Promise.all(appCalls);
}
