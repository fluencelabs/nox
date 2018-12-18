import {Network} from "../types/web3-contracts/Network";
const hexToArrayBuffer = require('hex-to-array-buffer');
const arrayBufferToHex = require('array-buffer-to-hex');

/**
 * Get the full status of Fluence contract from ethereum blockchain.
 * @param contract Fluence contract API
 */
export async function getStatus(contract: Network): Promise<Status> {
    let codes = await getCodes(contract);
    let ready_nodes = await getReadyNodes(contract);
    let clusters = await getClusters(contract);

    return {
        clusters: clusters,
        enqueued_codes: codes,
        ready_nodes: ready_nodes
    };
}

const IP_LEN = 4;

const TENDERMINT_KEY_LEN = 20;

interface NodeAddress {
    tendermint_key: string,
    ip_addr: string
}

/**
 * Decode node address to tendermint key and IP address.
 *
 */
function decodeNodeAddress(nodeAddress: string): NodeAddress {
    let buf = hexToArrayBuffer(nodeAddress.replace("0x", ""));

    let tendermint_key = arrayBufferToHex(buf.slice(0, TENDERMINT_KEY_LEN));

    let ip_buf = new DataView(buf.slice(TENDERMINT_KEY_LEN, TENDERMINT_KEY_LEN + IP_LEN));

    let ip_addr = `${ip_buf.getUint8(0)}.${ip_buf.getUint8(1)}.${ip_buf.getUint8(2)}.${ip_buf.getUint8(3)}`;

    return {tendermint_key: tendermint_key, ip_addr: ip_addr};
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
        let addr = decodeNodeAddress(unparsedNodes["1"][index]);
        let node: Node = {
            id: id,
            tendermint_key: addr.tendermint_key,
            ip_addr: addr.ip_addr,
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
            let addr = decodeNodeAddress(unparsedClustersNodes["1"][index]);
            let member: ClusterMember = {
                id: unparsedClustersNodes["0"][members_counter],
                tendermint_key: addr.tendermint_key,
                ip_addr: addr.ip_addr,
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
