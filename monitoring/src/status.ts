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
import {getReadyNodes, Node} from "./node";
import {Code, getEnqueuedCodes} from "./code";
import {Cluster, getClusters} from "./cluster";

export interface Status {
    clusters: Cluster[],
    enqueued_codes: Code[],
    ready_nodes: Node[]
}

/**
 * Get the full status of Fluence contract from ethereum blockchain.
 * @param contract Fluence contract API
 */
export async function getStatus(contract: Network): Promise<Status> {
    let codes = await getEnqueuedCodes(contract);
    let readyNodes = await getReadyNodes(contract);
    let clusters = await getClusters(contract);

    return {
        clusters: clusters,
        enqueued_codes: codes,
        ready_nodes: readyNodes
    };
}
