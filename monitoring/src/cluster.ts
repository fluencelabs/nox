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
import {decodeNodeAddress} from "./nodeAddress";
import {App, parseCodes} from "./app";

export interface ClusterMember {
    id: string,
    port: number
}

export interface Cluster {
    id: string,
    genesis_time: number,
    app: App,
    cluster_members: ClusterMember[]
}

/**
 * Gets list of formed clusters from Fluence contract
 */
export async function getClusters(contract: Network): Promise<Cluster[]> {

    let allIds = await contract.methods.getIds().call();

    let clusterIds = allIds["1"];

    let clusterCalls: Promise<Cluster>[] = clusterIds.map((id, _) => {
        return contract.methods.getCluster(id).call().then((res) => {
            let addr = decodeNodeAddress(res["0"]);
            let app: App = {
                app_address: res["0"],
                storage_receipt: res["1"],
                cluster_size: parseInt(res["2"]),
                developer: res["3"],
                pinToNodes: res["4"]
            };

            let cluster_members: ClusterMember[] = res["6"].map((member_id, idx) => {
                return {
                    id: member_id,
                    port: parseInt(res["7"][idx])
                }
            });

            return {
                id: id,
                genesis_time: parseInt(res["5"]),
                app: app,
                cluster_members: cluster_members
            };
        });
    });

    return Promise.all(clusterCalls);
}