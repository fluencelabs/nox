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
import {App} from "./app";

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
export async function getClusters(contract: Network, ids: string[]): Promise<Cluster[]> {

    let clusterCalls: Promise<Cluster>[] = ids.map((id) => {
        return contract.methods.getCluster(id).call().then((res) => {

            let appAddress = res["0"];
            let storageReceipt = res["1"];
            let clusterSize = parseInt(res["2"]);
            let developer = res["3"];
            let pinToNodes = res["4"];

            let app: App = {
                app_address: appAddress,
                storage_receipt: storageReceipt,
                cluster_size: clusterSize,
                developer: developer,
                pinToNodes: pinToNodes
            };

            let memberIds = res["6"];
            let memberPorts = res["7"];

            let cluster_members: ClusterMember[] = memberIds.map((member_id, idx) => {
                return {
                    id: member_id,
                    port: parseInt(memberPorts[idx])
                }
            });

            let genesisTime = parseInt(res["5"]);

            return {
                id: id,
                genesis_time: genesisTime,
                app: app,
                cluster_members: cluster_members
            };
        });
    });

    return Promise.all(clusterCalls);
}
