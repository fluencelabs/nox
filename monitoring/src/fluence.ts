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
}

async function getStatus(contract: Network): Promise<Status> {
    let codes = await getCodes(contract);
    let ready_nodes = await getReadyNodes(contract);
    let clusters = await getClusters(contract);

    return {
        clusters: clusters,
        enqueued_codes: codes,
        ready_nodes: ready_nodes
    };
}

async function getCodes(contract: Network): Promise<Code[]> {
    let unparsedCodes: { "0": string[]; "1": string[]; "2": string[] } = await contract.methods.getEnqueuedCodes().call();
    return parseCodes(unparsedCodes);
}

function parseCodes(unparsed: { "0": string[]; "1": string[]; "2": string[] }): Code[] {
    let codes: Code[] = [];
    unparsed["0"].forEach((hash, index) => {
        let code: Code = {
            storage_hash: hash,
            storage_receipt: unparsed["1"][index],
            cluster_size: parseInt(unparsed["2"][index])
        };
        codes.push(code);
    });
    return codes;
}

async function getReadyNodes(contract: Network): Promise<Node[]> {
    let unparsedNodes = await contract.methods.getReadyNodes().call();
    let nodes: Node[] = [];
    unparsedNodes["0"].forEach((id, index) => {
        let node: Node = {
            id: id,
            tendermint_key: unparsedNodes["1"][index],
            ip_addr: unparsedNodes["1"][index],
            start_port: parseInt(unparsedNodes["2"][index]),
            end_port: parseInt(unparsedNodes["3"][index]),
            current_port: parseInt(unparsedNodes["4"][index])
        };
        nodes.push(node);
    });
    return nodes;
}

async function getClusters(contract: Network): Promise<Cluster[]> {
    let unparsedClustersInfo = await contract.methods.getClustersInfo().call();
    let unparsedClustersNodes = await contract.methods.getClustersNodes().call();
    let clusters: Cluster[] = [];
    let codes: Code[] = parseCodes({"0": unparsedClustersInfo["2"],
        "1": unparsedClustersInfo["3"],
        "2": unparsedClustersInfo["4"]
    });
    var members_counter = 0;
    unparsedClustersInfo["0"].forEach((id, index) => {

        let code = codes[index];

        let clustersMembers: ClusterMember[] = [];

        for(var i = 0; i < code.cluster_size; i++){
            let member: ClusterMember = {
                id: unparsedClustersNodes["0"][members_counter],
                tendermint_key: unparsedClustersNodes["1"][members_counter],
                ip_addr: unparsedClustersNodes["1"][members_counter],
                port: parseInt(unparsedClustersNodes["2"][members_counter])
            };
            clustersMembers.push(member);
            members_counter++;
        }

        let cluster: Cluster = {
            id: unparsedClustersInfo["0"][index],
            genesis_time: parseInt(unparsedClustersInfo["1"][index]),
            code: code,
            cluster_members: clustersMembers
        };
        clusters.push(cluster);
    });
    return clusters;
}

export interface Status {
    clusters: Cluster[],
    enqueued_codes: Code[],
    ready_nodes: Node[]
}

export interface Node {
    id: string,
    tendermint_key: string,
    ip_addr: string,
    start_port: number,
    end_port: number,
    current_port: number
}

export interface Code {
    storage_hash: string,
    storage_receipt: string,
    cluster_size: number
}

export interface ClusterMember {
    id: string,
    tendermint_key: string,
    ip_addr: string,
    port: number
}

export interface Cluster {
    id: string,
    genesis_time: number,
    code: Code,
    cluster_members: ClusterMember[]
}


