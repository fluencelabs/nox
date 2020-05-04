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

export interface Address {
    protocols: Protocol[]
}

export interface Protocol {
    protocol: ProtocolType,
    value?: string
}

export enum ProtocolType {
    Service = "service",
    Peer = "peer",
    Client = "client"
}

export function addressToString(address: Address): string {
    let addressStr = "fluence:";

    for (let addr of address.protocols) {
        addressStr = addressStr + "/" + addr.protocol;
        if (addr.value) {
            addressStr = addressStr + "/" + addr.value;
        }
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
        case ProtocolType.Service:
            return protocolWithValue(protocol, protocolIterator);
        case ProtocolType.Client:
            return protocolWithValue(protocol, protocolIterator);
        case ProtocolType.Peer:
            return protocolWithValue(protocol, protocolIterator);
        default:
            throw Error("cannot parse protocol. Should be 'service|peer|client'");
    }

}

export function createRelayAddress(relay: string, client: string): Address {
    let protocols = [{protocol: ProtocolType.Peer, value: relay}, {protocol: ProtocolType.Client, value: client}];

    return {
        protocols: protocols
    }
}

export function createServiceAddress(service: string): Address {

    let protocol = { protocol: ProtocolType.Service, value: service };

    return {
        protocols: [protocol]
    }
}

export function createPeerAddress(peer: string): Address {
    let protocol = { protocol: ProtocolType.Peer, value: peer };

    return {
        protocols: [protocol]
    }
}

export function parseAddress(str: string): Address {
    str = str.replace("fluence:", "");

    // delete leading slashes
    str = str.replace(/^\/+/, '');

    let parts = str.split("/");
    if (parts.length < 1) {
        throw Error("address parts should not be empty")
    }

    let protocols: Protocol[] = [];
    let partsEntries: IterableIterator<[number, string]> = parts.entries();

    while(true) {
        let result = partsEntries.next();
        if (result.done) break;
        let protocol = parseProtocol(result.value[1], partsEntries);
        protocols.push(protocol);
    }

    return {
        protocols: protocols
    }
}
