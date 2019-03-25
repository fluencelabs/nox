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
import JSONFormatter from 'json-formatter-js';
import * as App from "./app"
import {getNodes, Node} from "./node";
import {Option} from "ts-option";
import Web3 = require('web3');
import abi = require("./Network.json");

let web3 = (window as any).web3;

export {
    Node as Node
}

/**
 * Contract status and status of all nodes in the Fluence network.
 */
export interface Status {
    contract_status: ContractStatus,
    node_statuses: any[]
}

function isDefined(str?: string): str is string {
    return str !== undefined;
}

/*
 * Gets Fluence Contract
 */
export function getContract(address: string, ethereumUrl?: string): Network {
    let web3js;
    if (isDefined(ethereumUrl)) {
        console.log('Connecting web3 to ' + ethereumUrl);
        web3js = new Web3(new Web3.providers.HttpProvider(ethereumUrl));
    } else if (typeof web3 === 'undefined') {
        console.log('Connecting web3 to default local node: http://localhost8545/');
        web3js = new Web3(new Web3.providers.HttpProvider("http://localhost8545/"));
    } else {
        // Use Mist/MetaMask's provider
        console.log("Using provided web3 (Mist/Metamask/etc)");
        web3js = new Web3(web3.currentProvider);
    }

    return new web3js.eth.Contract(abi, address) as Network;
}

/*
 * Gets nodes that are members of a cluster with a specific app (by appId).
 */
export async function getAppNodes(contractAddress: string, appId: string, ethereumUrl?: string): Promise<Node[]> {

    let contract = getContract(contractAddress, ethereumUrl);

    // get app info from contract
    let app = await App.getApp(contract, appId);

    let cluster = app.cluster;

    let result: Option<Promise<Node[]>> = cluster.map((c) => {

        // get info about all node members of the app
        return getNodes(contract, c.node_ids).then((nodes) => {
            // combine nodes with a specific port in the app
            return nodes;
        });
    });

    return result.getOrElse(Promise.resolve([]));
}

// get node health status by HTTP
export function getNodeStatus(node: Node): Promise<any> {
    let url = `http://${node.ip_addr}:${node.api_port}/status`;
    return axios.get(url).then((res) => {
        return res.data;
    }).catch((err) => {
        return {
            nodeInfo: node,
            causeBy: err
        };
    });
}

// gets status of the worker through API
export function getWorkerStatus(apiAddr: string, apiPort: string, appId: number): Promise<any> {
    let url = `http://${apiAddr}:${apiPort}/apps/${appId}/status`;
    return axios.get(url).then((res) => {
        return res.data;
    }).catch((err) => {
        return {
            apiAddr: apiAddr,
            apiPort: apiPort,
            causeBy: err
        };
    });
}

/**
 * Shows status of Fluence contract on the page.
 * @param ethereumUrl Url of an Ethereum node
 * @param contractAddress Address from ganache by default. todo: use address from mainnet as default
 */
export async function getStatus(contractAddress: string, ethereumUrl?: string): Promise<Status> {

    let contract = getContract(contractAddress, ethereumUrl, );

    let contractStatus = await getContractStatus(contract);

    let responses = contractStatus.nodes.map((node) => {
       return getNodeStatus(node);
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
    let status = getStatus(contractAddress, undefined);
    status.then((st) => {
        const formatter = new JSONFormatter(st);
        document.body.innerHTML = '';
        document.body.appendChild(formatter.render());
        formatter.openAtDepth(3);
    });
}

const _global = (window /* browser */ || global /* node */) as any;
_global.showStatus = showStatus;
