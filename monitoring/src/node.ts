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
 * The purpose of real-time nodes is to host developer's [`Code`], e.g., backend code.
 */
export interface Node {
    id: string,
    tendermint_key: string,
    ip_addr: string,
    start_port: number,
    end_port: number,
    current_port: number
}

/**
 * Gets list of ready-to-work nodes from Fluence contract
 */
export async function getReadyNodes(contract: Network): Promise<Node[]> {
    let unparsedNodes = await contract.methods.getReadyNodes().call();
    let nodes: Node[] = [];

    let ids = unparsedNodes["0"];
    let addresses = unparsedNodes["1"];
    let startPorts = unparsedNodes["2"];
    let endPorts = unparsedNodes["3"];
    let currentPorts = unparsedNodes["4"];

    ids.forEach((id, index) => {
        let addr = decodeNodeAddress(addresses[index]);
        let node: Node = {
            id: id,
            tendermint_key: addr.tendermint_key,
            ip_addr: addr.ip_addr,
            start_port: parseInt(startPorts[index]),
            end_port: parseInt(endPorts[index]),
            current_port: parseInt(currentPorts[index])
        };
        nodes.push(node);
    });
    return nodes;
}
