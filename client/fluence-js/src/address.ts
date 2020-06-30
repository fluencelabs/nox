/*
 * Copyright 2020 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

import * as PeerId from "peer-id";
import {encode} from "bs58"

export interface Address {
    protocols: Protocol[],
    hash?: string
}

export interface Protocol {
    protocol: ProtocolType,
    value?: string
}

export enum ProtocolType {
    Providers = "providers",
    Peer = "peer",
    Signature = "signature",
    Client = "client"
}

const PROTOCOL = "fluence:";

export function addressToString(address: Address): string {
    let addressStr = PROTOCOL;

    for (let addr of address.protocols) {
        addressStr = addressStr + "/" + addr.protocol;
        if (addr.value) {
            addressStr = addressStr + "/" + addr.value;
        }
    }

    if (address.hash) {
        addressStr = addressStr + "#" + address.hash
    }

    return addressStr;
}

function protocolWithValue(protocol: ProtocolType, protocolIterator: IterableIterator<[number, string]>): Protocol {

    let protocolValue = protocolIterator.next().value;

    if (!protocolValue || !protocolValue[1]) {
        throw Error(`protocol '${protocol}' should be with a value`)
    }

    return {protocol: protocol, value: protocolValue[1]};
}


export function parseProtocol(protocol: string, protocolIterator: IterableIterator<[number, string]>): Protocol {
    protocol = protocol.toLocaleLowerCase();

    switch (protocol) {
        case ProtocolType.Providers:
            return protocolWithValue(protocol, protocolIterator);
        case ProtocolType.Client:
            return protocolWithValue(protocol, protocolIterator);
        case ProtocolType.Peer:
            return protocolWithValue(protocol, protocolIterator);
        case ProtocolType.Signature:
            return protocolWithValue(protocol, protocolIterator);
        default:
            throw Error("cannot parse protocol. Should be 'service|peer|client|signature'");
    }

}

export async function createRelayAddress(relay: string, peerId: PeerId, withSig: boolean, hash?: string): Promise<Address> {

    let protocols = [
        {protocol: ProtocolType.Peer, value: relay},
        {protocol: ProtocolType.Client, value: peerId.toB58String()}
    ];

    if (withSig) {
        let str = addressToString({protocols: protocols}).replace(PROTOCOL, "");
        let signature = await peerId.privKey.sign(Buffer.from(str));
        let signatureStr = encode(signature);

        protocols.push({protocol: ProtocolType.Signature, value: signatureStr});
    }

    return {
        protocols: protocols,
        hash: hash
    }
}

export function createServiceAddress(service: string, hash?: string): Address {

    let protocol = {protocol: ProtocolType.Providers, value: service};

    return {
        protocols: [protocol],
        hash: hash
    }
}

export function createPeerAddress(peer: string, hash?: string): Address {
    let protocol = {protocol: ProtocolType.Peer, value: peer};

    return {
        protocols: [protocol],
        hash: hash
    }
}

export function parseAddress(str: string): Address {
    str = str.replace("fluence:", "");

    // delete leading slashes
    str = str.replace(/^\/+/, '');

    let mainAndHash = str.split("#");

    let parts = mainAndHash[0].split("/");
    if (parts.length < 1) {
        throw Error("address parts should not be empty")
    }

    let protocols: Protocol[] = [];
    let partsEntries: IterableIterator<[number, string]> = parts.entries();

    while (true) {
        let result = partsEntries.next();
        if (result.done) break;
        let protocol = parseProtocol(result.value[1], partsEntries);
        protocols.push(protocol);
    }

    let hashPart = mainAndHash.slice(1, mainAndHash.length).join();
    let hash = undefined;
    if (hashPart) {
        hash = hashPart;
    }

    return {
        protocols: protocols,
        hash: hash
    }
}
