/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
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
