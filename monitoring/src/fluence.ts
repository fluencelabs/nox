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

import {ContractStatus, getContractStatus} from "./contractStatus";
import axios from 'axios';
import {Network} from "../types/web3-contracts/Network";
import {NodeStatus, UnavailableNode} from "./nodeStatus";
import JSONFormatter from 'json-formatter-js';
import abi = require("./Network.json");
import Web3 = require('web3');

(window as any).web3 = (window as any).web3 || {};
let web3 = (window as any).web3;

/**
 * Contract status and status of all nodes in the Fluence network.
 */
export interface Status {
    contract_status: ContractStatus,
    node_statuses: (NodeStatus|UnavailableNode)[]
}

/**
 * Shows status of Fluence contract on the page.
 * @param contractAddress address from ganache by default. todo: use address from mainnet as default
 */
export async function getStatus(contractAddress: string): Promise<Status> {

    let web3js;
    if (typeof web3 !== 'undefined') {
        // Use Mist/MetaMask's provider
        web3js = new Web3(web3.currentProvider);
    } else {
        console.log('No web3? Trying to connect to the local node!');
        web3js = new Web3(new Web3.providers.HttpProvider("http://localhost:8545"));
    }

    let contract: Network = new web3js.eth.Contract(abi, contractAddress) as Network;

    let contractStatus = await getContractStatus(contract);

    let responses = contractStatus.nodes.map((node) => {
        // todo: `+400` is a temporary solution, fix it after implementing correct port management
        let url = `http://${node.ip_addr}:${node.last_port + 400}/status`;
        return axios.get(url).then((res) => {
            res.data.status = "ok";
            return <NodeStatus>res.data;
        }).catch(() => {
            return {nodeInfo: node, status: "unavailable"}
        });
    });

    let nodeStatuses = await Promise.all(responses);

    return {
        contract_status: contractStatus,
        node_statuses: nodeStatuses
    };

}

/**
 * Show rendered status of Fluence network.
 */
export function showStatus(contractAddress: string = "0x9995882876ae612bfd829498ccd73dd962ec950a") {
    let status = getStatus(contractAddress);
    status.then((st) => {
        const formatter = new JSONFormatter(st);
        document.body.appendChild(formatter.render());
    });
}

const _global = (window /* browser */ || global /* node */) as any;
_global.showStatus = showStatus;
