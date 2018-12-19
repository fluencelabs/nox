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
import {abi} from "./abi";
import {NodeStatus} from "./nodeStatus";
import JSONFormatter from 'json-formatter-js';
import Web3 = require('web3');

(window as any).web3 = (window as any).web3 || {};
let web3 = (window as any).web3;

interface Status {
    contract_status: ContractStatus,
    node_statuses: NodeStatus[]
}

interface StatusAddress {
    ipAddr: string,
    port: number
}

/**
 * Shows status of Fluence contract on the page.
 * @param contractAddress address from ganache by default. todo: use address from mainnet as default
 */
export async function getStatus(contractAddress: string): Promise<Status> {

    let web3js;
    if (typeof web3 !== 'undefined') {
        // Use Mist/MetaMask's provider
        console.log('MetaMask will be used!');
        web3js = new Web3(web3.currentProvider);
    } else {
        console.log('No web3? Trying to connect to the local node!');
        web3js = new Web3(new Web3.providers.HttpProvider("http://localhost:8545"));
    }

    let accounts = await web3js.eth.getAccounts();

    let contract: Network = new web3js.eth.Contract(abi, contractAddress) as Network;

    let contractStatus = await getContractStatus(contract);

    let statusEndpoints: StatusAddress[] = contractStatus.ready_nodes.map((node) => {
        let statusPort = node.start_port + 400;
        let ipAddr = node.ip_addr;
        return {ipAddr: ipAddr, port: statusPort};
    });

    let responses = statusEndpoints.map((enpd) => {
        let url = `http://${enpd.ipAddr}:${enpd.port}/status`;
        return axios.get(url).then((res) => {
            return <NodeStatus>res.data;
        });
    });

    let nodeStatuses = await Promise.all(responses);

    return {
        contract_status: contractStatus,
        node_statuses: nodeStatuses
    };

}

export function showStatus(contractAddress: string = "0x9995882876ae612bfd829498ccd73dd962ec950a") {
    let status = getStatus(contractAddress);
    status.then((st) => {
        const formatter = new JSONFormatter(st);
        document.body.appendChild(formatter.render());
    });

}

const _global = (window /* browser */ || global /* node */) as any;
_global.showStatus = showStatus;