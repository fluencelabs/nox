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
import {Code, parseCodesFromClustersInfos} from "./code";

export type ClustersInfos = { "0": string[]; "1": string[]; "2": string[]; "3": string[]; "4": string[] };

export interface ClusterMember {
    id: string,
    tendermint_key: string,
    ip_addr: string,
    port: number
}

export interface Cluster {
    id: string,
    genesis_time: number,
    code: Code,
    cluster_members: ClusterMember[]
}

/**
 * Gets list of formed clusters from Fluence contract
 */
export async function getClusters(contract: Network): Promise<Cluster[]> {
    let unparsedClustersInfo = await contract.methods.getClustersInfo().call();
    let unparsedClustersNodes = await contract.methods.getClustersNodes().call();
    let clusters: Cluster[] = [];
    let codes: Code[] = parseCodesFromClustersInfos(unparsedClustersInfo);
    var membersCounter = 0;

    unparsedClustersInfo["0"].forEach((id, index) => {

        let code = codes[index];

        let clustersMembers: ClusterMember[] = [];

        for(var i = 0; i < code.cluster_size; i++){
            let addr = decodeNodeAddress(unparsedClustersNodes["1"][index]);
            let member: ClusterMember = {
                id: unparsedClustersNodes["0"][membersCounter],
                tendermint_key: addr.tendermint_key,
                ip_addr: addr.ip_addr,
                port: parseInt(unparsedClustersNodes["2"][membersCounter])
            };
            clustersMembers.push(member);
            membersCounter++;
        }

        let cluster: Cluster = {
            id: unparsedClustersInfo["0"][index],
            genesis_time: parseInt(unparsedClustersInfo["1"][index]),
            code: code,
            cluster_members: clustersMembers
        };
        clusters.push(cluster);
    });
    return clusters;
}