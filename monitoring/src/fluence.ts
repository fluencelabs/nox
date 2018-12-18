import {getStatus} from "./ContractStatus";

(window as any).web3 = (window as any).web3 || {};

import Web3 = require('web3');
import {Network} from "../types/web3-contracts/Network";
import {abi} from "./abi";

/**
 * Shows status of Fluence contract on the page.
 */
export async function show_status(contract_address: string = "0x9995882876ae612bfd829498ccd73dd962ec950a"): Promise<void> {

    let web3js = new Web3(new Web3.providers.HttpProvider("http://localhost:8545"));

    let accounts = await web3js.eth.getAccounts();

    let contract: Network = new web3js.eth.Contract(abi, contract_address) as Network;

    let status = await getStatus(contract);
    document.body.innerHTML += `<div><pre>${JSON.stringify(status, undefined, 2)}</pre></div>`;
}

const _global = (window /* browser */ || global /* node */) as any;
_global.show_status = show_status;