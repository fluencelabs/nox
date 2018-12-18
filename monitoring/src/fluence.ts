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

import {getStatus} from "./status";

(window as any).web3 = (window as any).web3 || {};

import Web3 = require('web3');
import {Network} from "../types/web3-contracts/Network";
import {abi} from "./abi";

/**
 * Shows status of Fluence contract on the page.
 */
export async function show_status(contract_address: string = "0x9995882876ae612bfd829498ccd73dd962ec950a"): Promise<void> {

    // TODO add metamask support
    let web3js = new Web3(new Web3.providers.HttpProvider("http://localhost:8545"));

    let accounts = await web3js.eth.getAccounts();

    let contract: Network = new web3js.eth.Contract(abi, contract_address) as Network;

    let status = await getStatus(contract);
    document.body.innerHTML += `<div><pre>${JSON.stringify(status, undefined, 2)}</pre></div>`;

    // TODO add statuses from nodes
}

const _global = (window /* browser */ || global /* node */) as any;
_global.show_status = show_status;