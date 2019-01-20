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
import * as App from "./app"
import {getNodes} from "./node";
import Web3 = require('web3');
import abi = require("./Network.json");
import {Option} from "ts-option";

(window as any).web3 = (window as any).web3 || {};
let web3 = (window as any).web3;

/**
 * Contract status and status of all nodes in the Fluence network.
 */
export interface Status {
    contract_status: ContractStatus,
    node_statuses: (NodeStatus|UnavailableNode)[]
}

export interface NodeAddress {
    ip: string,
    port: number
}

export function getContract(address: string): Network {
    let web3js;
    if (typeof web3 !== 'undefined') {
        // Use Mist/MetaMask's provider
        web3js = new Web3(web3.currentProvider);
    } else {
        console.log('No web3? Trying to connect to the local node!');
        web3js = new Web3(new Web3.providers.HttpProvider("http://localhost:8545"));
    }

    return new web3js.eth.Contract(abi, address) as Network;
}

export async function getNodeAddresses(contractAddress: string, appId: string): Promise<NodeAddress[]> {

    let contract = getContract(contractAddress);

    let app = await App.getApp(contract, appId);

    let cluster = app.cluster;

    let result: Option<Promise<NodeAddress[]>> = cluster.map((c) => {

        let ids: string[] = c.cluster_members.map((m) => m.id);

        return getNodes(contract, ids).then((nodes) => {
            return nodes.map((n, idx) => {
                return {
                    ip: n.ip_addr,
                    port: c.cluster_members[idx].port
                }
            })
        });
    });

    return result.getOrElse(Promise.resolve([]));
}

/**
 * Shows status of Fluence contract on the page.
 * @param contractAddress address from ganache by default. todo: use address from mainnet as default
 */
export async function getStatus(contractAddress: string): Promise<Status> {

    let contract = getContract(contractAddress);

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
export function showStatus(contractAddress: string) {
    let status = getStatus(contractAddress);
    status.then((st) => {
        const formatter = new JSONFormatter(st);
        document.body.appendChild(formatter.render());
    });
}

const _global = (window /* browser */ || global /* node */) as any;
_global.showStatus = showStatus;
