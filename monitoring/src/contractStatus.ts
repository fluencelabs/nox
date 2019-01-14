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
import {getNodes, Node} from "./node";
import {App, getEnqueuedApps} from "./app";
import {Cluster, getClusters} from "./cluster";

/**
 * Status about Fluence network from ethereum contract.
 */
export interface ContractStatus {
    clusters: Cluster[],
    enqueued_codes: App[],
    nodes: Node[]
}

/**
 * Get the full status of Fluence contract from ethereum blockchain.
 * @param contract Fluence contract API
 */
export async function getContractStatus(contract: Network): Promise<ContractStatus> {

    let nodesIds = await contract.methods.getNodesIds().call();

    let clustersIds = await contract.methods.getClustersIds().call();

    let codes = await getEnqueuedApps(contract);
    let nodes = await getNodes(contract, nodesIds);
    let clusters = await getClusters(contract, clustersIds);

    return {
        clusters: clusters,
        enqueued_codes: codes,
        nodes: nodes
    };
}
