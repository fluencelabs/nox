import {getStatus} from "./ContractStatus";

let abi = [{"constant":true,"inputs":[{"name":"_operator","type":"address"},{"name":"_role","type":"string"}],"name":"checkRole","outputs":[],"payable":false,"stateMutability":"view","type":"function"},{"constant":true,"inputs":[],"name":"ROLE_WHITELISTED","outputs":[{"name":"","type":"string"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":true,"inputs":[{"name":"_operator","type":"address"},{"name":"_role","type":"string"}],"name":"hasRole","outputs":[{"name":"","type":"bool"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":false,"inputs":[{"name":"storageHash","type":"bytes32"},{"name":"storageReceipt","type":"bytes32"},{"name":"clusterSize","type":"uint8"}],"name":"addCode","outputs":[],"payable":false,"stateMutability":"nonpayable","type":"function"},{"constant":false,"inputs":[{"name":"_operators","type":"address[]"}],"name":"removeAddressesFromWhitelist","outputs":[],"payable":false,"stateMutability":"nonpayable","type":"function"},{"constant":false,"inputs":[{"name":"_operator","type":"address"}],"name":"removeAddressFromWhitelist","outputs":[],"payable":false,"stateMutability":"nonpayable","type":"function"},{"constant":true,"inputs":[],"name":"getStatus","outputs":[{"name":"","type":"bytes32[]"},{"name":"","type":"bytes32[]"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":false,"inputs":[],"name":"renounceOwnership","outputs":[],"payable":false,"stateMutability":"nonpayable","type":"function"},{"constant":false,"inputs":[{"name":"_operator","type":"address"}],"name":"addAddressToWhitelist","outputs":[],"payable":false,"stateMutability":"nonpayable","type":"function"},{"constant":false,"inputs":[{"name":"nodeID","type":"bytes32"},{"name":"nodeAddress","type":"bytes24"},{"name":"startPort","type":"uint16"},{"name":"endPort","type":"uint16"}],"name":"addNode","outputs":[],"payable":false,"stateMutability":"nonpayable","type":"function"},{"constant":true,"inputs":[],"name":"owner","outputs":[{"name":"","type":"address"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":true,"inputs":[],"name":"getClustersNodes","outputs":[{"name":"","type":"bytes32[]"},{"name":"","type":"bytes24[]"},{"name":"","type":"uint16[]"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":true,"inputs":[],"name":"getClustersInfo","outputs":[{"name":"","type":"bytes32[]"},{"name":"","type":"uint256[]"},{"name":"","type":"bytes32[]"},{"name":"","type":"bytes32[]"},{"name":"","type":"uint8[]"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":true,"inputs":[{"name":"_operator","type":"address"}],"name":"whitelist","outputs":[{"name":"","type":"bool"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":true,"inputs":[],"name":"getReadyNodes","outputs":[{"name":"","type":"bytes32[]"},{"name":"","type":"bytes24[]"},{"name":"","type":"uint16[]"},{"name":"","type":"uint16[]"},{"name":"","type":"uint16[]"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":true,"inputs":[{"name":"nodeID","type":"bytes32"}],"name":"getNodeClusters","outputs":[{"name":"","type":"bytes32[]"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":true,"inputs":[{"name":"clusterID","type":"bytes32"}],"name":"getCluster","outputs":[{"name":"","type":"bytes32"},{"name":"","type":"bytes32"},{"name":"","type":"uint256"},{"name":"","type":"bytes32[]"},{"name":"","type":"bytes24[]"},{"name":"","type":"uint16[]"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":false,"inputs":[{"name":"_operators","type":"address[]"}],"name":"addAddressesToWhitelist","outputs":[],"payable":false,"stateMutability":"nonpayable","type":"function"},{"constant":false,"inputs":[{"name":"_newOwner","type":"address"}],"name":"transferOwnership","outputs":[],"payable":false,"stateMutability":"nonpayable","type":"function"},{"constant":true,"inputs":[],"name":"getEnqueuedCodes","outputs":[{"name":"","type":"bytes32[]"},{"name":"","type":"bytes32[]"},{"name":"","type":"uint8[]"}],"payable":false,"stateMutability":"view","type":"function"},{"anonymous":false,"inputs":[{"indexed":false,"name":"clusterID","type":"bytes32"},{"indexed":false,"name":"storageHash","type":"bytes32"},{"indexed":false,"name":"genesisTime","type":"uint256"},{"indexed":false,"name":"solverIDs","type":"bytes32[]"},{"indexed":false,"name":"solverAddrs","type":"bytes24[]"},{"indexed":false,"name":"solverPorts","type":"uint16[]"}],"name":"ClusterFormed","type":"event"},{"anonymous":false,"inputs":[{"indexed":false,"name":"storageHash","type":"bytes32"}],"name":"CodeEnqueued","type":"event"},{"anonymous":false,"inputs":[{"indexed":false,"name":"id","type":"bytes32"}],"name":"NewNode","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"operator","type":"address"},{"indexed":false,"name":"role","type":"string"}],"name":"RoleAdded","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"operator","type":"address"},{"indexed":false,"name":"role","type":"string"}],"name":"RoleRemoved","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"previousOwner","type":"address"}],"name":"OwnershipRenounced","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"previousOwner","type":"address"},{"indexed":true,"name":"newOwner","type":"address"}],"name":"OwnershipTransferred","type":"event"}];

(window as any).web3 = (window as any).web3 || {};

import Web3 = require('web3');
import {Network} from "../types/web3-contracts/Network";


window.addEventListener('load', function() {

    app();

});

export async function app(): Promise<void> {

    console.log("on load");

    var web3js;

    console.log((window as any));

    // Checking if Web3 has been injected by the browser (Mist/MetaMask)
    if (typeof (window as any) !== 'undefined') {

        console.log("non undefined");
        // Use Mist/MetaMask's provider
        // web3js = new Web3(window.web3.currentProvider);
        web3js = new Web3(new Web3.providers.HttpProvider("http://localhost:8545"));
    } else {
        console.log('No web3? You should consider trying MetaMask!');
        // fallback - use your fallback strategy (local node / hosted node + in-dapp id mgmt / fail)
        web3js = new Web3(new Web3.providers.HttpProvider("http://localhost:8545"));
    }

    // Now you can start your app & access web3 freely:

    console.log("on web3js " + web3js);
    let accounts = await web3js.eth.getAccounts();
    console.log("accounts: " + accounts);

    var coinbase = await web3js.eth.getCoinbase();
    console.log("after coinbase");
    var originalBalance = await web3js.eth.getBalance(accounts[0]);


    console.log("balance === " + JSON.stringify(originalBalance));

    let contract: Network = new web3js.eth.Contract(abi, "0x9995882876ae612bfd829498ccd73dd962ec950a") as Network;

    let status = await getStatus(contract);
    console.log(status);

    let addrs_all: string[] = [];
    status.clusters.forEach((cl) => {
        cl.cluster_members.forEach((clm => {
            addrs_all.push(clm.ip_addr);
        }))
    });

    status.ready_nodes.forEach((n) => {
        addrs_all.push(n.ip_addr);
    });

    let addrs: string[] =  Array.from(new Set(addrs_all));

    console.log(JSON.stringify(addrs));
    console.log(JSON.stringify(addrs_all));
}
