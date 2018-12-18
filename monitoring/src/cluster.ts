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
import {Code, parseCodes} from "./code";

export interface ClusterMember {
    id: string,
    tendermint_key: string,
    ip_addr: string,
    port: number,
    owner: string
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

    let cluster_ids = unparsedClustersInfo["0"];
    let genesis_times = unparsedClustersInfo["1"];

    let codeAddresses = unparsedClustersInfo["2"];
    let storageReceipts = unparsedClustersInfo["3"];
    let clusterSizes = unparsedClustersInfo["4"];
    let developers = unparsedClustersInfo["5"];
    let codes: Code[] = parseCodes(codeAddresses, storageReceipts, clusterSizes, developers);

    let node_ids = unparsedClustersNodes["0"];
    let node_addresses = unparsedClustersNodes["1"];
    let node_ports = unparsedClustersNodes["2"];
    let node_owners = unparsedClustersNodes["3"];

    var membersCounter = 0;
    cluster_ids.forEach((id, index) => {

        let code = codes[index];

        let clustersMembers: ClusterMember[] = [];

        for(var i = 0; i < code.cluster_size; i++){
            let addr = decodeNodeAddress(node_addresses[index]);
            let member: ClusterMember = {
                id: node_ids[membersCounter],
                tendermint_key: addr.tendermint_key,
                ip_addr: addr.ip_addr,
                port: parseInt(node_ports[membersCounter]),
                owner: node_owners[membersCounter]
            };
            clustersMembers.push(member);
            membersCounter++;
        }

        let cluster: Cluster = {
            id: cluster_ids[index],
            genesis_time: parseInt(genesis_times[index]),
            code: code,
            cluster_members: clustersMembers
        };
        clusters.push(cluster);
    });
    return clusters;
}