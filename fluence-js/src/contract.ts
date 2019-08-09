import axios from "axios";
import Web3 from "web3";
import hexToArrayBuffer from "hex-to-array-buffer";
import arrayBufferToHex from "array-buffer-to-hex";
import abi from "./Network.json";
import { Network } from "../types/web3-contracts/Network";

interface NodeAddress {
    tendermint_key: string,
    ip_addr: string
}

export interface Node {
    id: string,
    tendermint_key: string,
    ip_addr: string,
    api_port: number,
    capacity: number,
    owner: string,
    is_private: boolean,
    clusters_ids: string[]
}

function getContract(address: string, ethereumUrl?: string): Network {
    let web3js;
    if (typeof ethereumUrl !== 'undefined') {
        console.log('Connecting web3 to ' + ethereumUrl);
        web3js = new Web3(new Web3.providers.HttpProvider(ethereumUrl));
    } else if (typeof window  === 'undefined' || typeof (window as any).web3 === 'undefined') {
        console.log('Connecting web3 to default local node: http://localhost:8545/');
        web3js = new Web3(new Web3.providers.HttpProvider("http://localhost:8545/"));
    } else {
        // Use Mist/MetaMask's provider
        console.log("Using provided web3 (Mist/Metamask/etc)");
        web3js = new Web3((window as any).web3.currentProvider);
    }

    return new web3js.eth.Contract(abi, address) as Network;
}

function decodeNodeAddress(nodeAddress: string): NodeAddress {
    const IP_LEN = 4;
    const TENDERMINT_KEY_LEN = 20;

    let buf = hexToArrayBuffer(nodeAddress.replace("0x", ""));

    let tendermint_key = arrayBufferToHex(buf.slice(0, TENDERMINT_KEY_LEN));

    let ip_buf = new DataView(buf.slice(TENDERMINT_KEY_LEN, TENDERMINT_KEY_LEN + IP_LEN));

    let ip_addr = `${ip_buf.getUint8(0)}.${ip_buf.getUint8(1)}.${ip_buf.getUint8(2)}.${ip_buf.getUint8(3)}`;

    return {
        tendermint_key: tendermint_key,
        ip_addr: ip_addr
    };
}

export async function getAppNodes(contractAddress: string, appId: string, ethereumUrl?: string): Promise<Node[]> {
    let contract = getContract(contractAddress, ethereumUrl);

    const unparsedApp = await contract.methods.getApp(appId).call();
    let nodeIds: string[] = unparsedApp["7"];

    return Promise.all(
        nodeIds.map(nodeId => contract.methods.getNode(nodeId).call()
            .catch((e) => {
                console.log(`Cannot get node ${nodeId}. Cause: ${e}`);
                throw e;
            })
            .then((res) => {
                let addr = decodeNodeAddress(res["0"]);
                let apiPort = parseInt(res["1"]);
                let capacity = parseInt(res["2"]);
                let owner = res["3"];
                let isPrivate = res["4"];
                let clusterIds = res["5"];

                return {
                    id: nodeId,
                    tendermint_key: addr.tendermint_key,
                    ip_addr: addr.ip_addr,
                    api_port: apiPort,
                    capacity: capacity,
                    owner: owner,
                    is_private: isPrivate,
                    clusters_ids: clusterIds
                };
            })
    ));
}

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
