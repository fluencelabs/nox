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

/**
 * Represents Fluence node registered in ethereum contract.
 * The node listens to contract events and runs real-time nodes.
 * The purpose of real-time nodes is to host developer's [`App`], e.g., backend code.
 */
export interface Node {
    id: string,
    tendermint_key: string,
    ip_addr: string,
    next_port: number,
    last_port: number,
    owner: string,
    is_private: boolean,
    clusters_ids: string[]
}

/**
 * Gets list of ready-to-work nodes from Fluence contract
 */
export async function getNodes(contract: Network): Promise<Node[]> {



    let allIds = await contract.methods.getIds().call();


    let nodeIds: string[] = allIds["0"];
    let clusterIds = allIds["1"];

    console.log("nodeids: " + nodeIds);

    let nodeCalls: Promise<Node>[] = nodeIds.map((id, _) => {
        return contract.methods.getNode(id).call().then((res) => {
            console.log(res);
            let addr = decodeNodeAddress(res["0"]);
            return {
                id: id,
                tendermint_key: addr.tendermint_key,
                ip_addr: addr.ip_addr,
                next_port: parseInt(res["1"]),
                last_port: parseInt(res["2"]),
                owner: res["3"],
                is_private: res["4"],
                clusters_ids: res["5"]
            };
        });
    });

    return Promise.all(nodeCalls);
}
