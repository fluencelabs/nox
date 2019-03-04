import { Network } from '../../../types/web3-contracts/Network';
import hexToArrayBuffer from 'hex-to-array-buffer';
import arrayBufferToHex from 'array-buffer-to-hex';
import axios from 'axios';
import {AppId} from "../apps";

export type NodeId = string;

/**
 * Represents Fluence node registered in ethereum contract.
 * The node listens to contract events and runs real-time nodes.
 * The purpose of real-time nodes is to host developer's [`App`], e.g., backend app.
 */
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

/**
 * Address of a node in Fluence contract. Contains tendermint key and IP address of the node.
 */
interface NodeAddress {
    tendermint_key: string,
    ip_addr: string
}

/**
 * Status of a Fluence node with all its workers.
 */
export interface NodeStatus {
    ip: string,
    listOfPorts: string,
    uptime: number,
    numberOfWorkers: number,
    workers: WorkerStatus[]
}

export interface NodeInfo {
    status: NodeStatus|null,
    nodeInfo: Node,
    causeBy: string|null
}

/**
 * If node is not available.
 */
export interface UnavailableNode {
    nodeInfo: Node,
    causeBy: string
}

/**
 * Status of a worker. It can exists but not available or turned off.
 */
export interface WorkerStatus {
    WorkerRunning?: WorkerRunning,
    WorkerContainerNotRunning?: WorkerContainerNotRunning
    WorkerNotYetLaunched?: WorkerNotYetLaunched
    WorkerHttpCheckFailed?: WorkerHttpCheckFailed
}

export interface WorkerRunning {
    info: WorkerInfo,
    uptime: number
}

export interface WorkerContainerNotRunning {
    info: WorkerInfo
}

export interface WorkerNotYetLaunched {
    info: WorkerInfo
}

export interface WorkerHttpCheckFailed {
    info: WorkerInfo,
    causedBy: string
}

export interface WorkerInfo {
    clusterId?: string,
    codeId: string,
    lastAppHash?: string,
    lastBlock?: string,
    lastBlockHeight?: number,
    p2pPort: number,
    rpcPort: number,
    stateMachinePrometheusPort?: number,
    tendermintPrometheusPort?: number
}

export async function getNodeIds(contract: Network): Promise<NodeId[]> {
    return contract.methods.getNodesIds().call();
}

export async function getNode(contract: Network, id: NodeId): Promise<Node> {
    return contract.methods.getNode(id).call().then((res) => {
        let addr = decodeNodeAddress(res["0"]);
        let apiPort = parseInt(res["1"]);
        let capacity = parseInt(res["2"]);
        let owner = res["3"];
        let isPrivate = res["4"];
        let clusterIds = res["5"];

        return {
            id: id,
            tendermint_key: addr.tendermint_key,
            ip_addr: addr.ip_addr,
            api_port: apiPort,
            capacity: capacity,
            owner: owner,
            is_private: isPrivate,
            clusters_ids: clusterIds
        };
    });
}

/**
 * Gets list of ready-to-work nodes from Fluence contract
 */
export async function getNodes(contract: Network, ids: NodeId[]): Promise<Node[]> {
    let nodeCalls: Promise<Node>[] = ids.map(id => getNode(contract, id));

    return Promise.all(nodeCalls);
}

/**
 * Decode node address to tendermint key and IP address.
 */
export function decodeNodeAddress(nodeAddress: string): NodeAddress {
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

export function getNodeStatus(node: Node): Promise<NodeInfo> {
    let url = `http://${node.ip_addr}:${node.api_port}/status`;
    return axios.get(url).then((res) => {
        return <NodeInfo>{
            status: <NodeStatus>res.data,
            nodeInfo: node,
            causeBy: null
        };
    }).catch((err) => {
        return {
            status: null,
            nodeInfo: node,
            causeBy: err
        };
    });
}

export interface NodeAppStatusInfo {
    status: NodeAppStatus|null,
    nodeInfo: Node,
    causeBy: string|null
}

export interface NodeAppStatus {
    node_info: NodeAppStatusAppInfo;
    sync_info: SyncInfo;
    validator_info: ValidatorInfo;
}
export interface NodeAppStatusAppInfo {
    protocol_version: ProtocolVersion;
    id: string;
    listen_addr: string;
    network: string;
    version: string;
    channels: string;
    moniker: string;
    other: Other;
}
export interface ProtocolVersion {
    p2p: string;
    block: string;
    app: string;
}
export interface Other {
    tx_index: string;
    rpc_address: string;
}
export interface SyncInfo {
    latest_block_hash: string;
    latest_app_hash: string;
    latest_block_height: string;
    latest_block_time: string;
    catching_up: boolean;
}
export interface ValidatorInfo {
    address: string;
    pub_key: PubKey;
    voting_power: string;
}
export interface PubKey {
    type: string;
    value: string;
}

/*
* TODO: move to JsonRPC client
*/
export function getNodeAppStatus(node: Node, appId: AppId): Promise<NodeAppStatusInfo> {
    let url = `http://${node.ip_addr}:${node.api_port}/apps/${appId}/status`;
    return axios.get(url, {
        timeout: 3000
    }).then((res: any) => {
        return <NodeAppStatusInfo>{
            status: <NodeAppStatus>res.data.result,
            nodeInfo: node,
            causeBy: null
        };
    }).catch((err) => {
        return {
            status: null,
            nodeInfo: node,
            causeBy: err
        };
    });
}
