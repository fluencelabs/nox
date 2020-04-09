/*
 * Copyright 2018 Fluence Labs Limited
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

export type Address = Relay | Service | Peer

export interface Relay {
    type: "Relay",
    relay: string,
    client: string
}

export interface Service {
    type: "Service",
    service: string
}

export interface Peer {
    type: "Peer",
    peer: string
}

export function createRelayAddress(relay: string, client: string): Relay {
    return {
        relay: relay,
        client: client,
        type: "Relay"
    }
}

export function createServiceAddress(service: string): Service {
    return {
        service: service,
        type: "Service"
    }
}

export function createPeerAddress(peer: string): Peer {
    return {
        type: "Peer",
        peer: peer
    }
}

/**
 * Check if all fields in json exist. It depends on type of address.
 * @param json
 */
export function parseAddressObj(json: any): Address {
    switch (json.type) {
        case "Relay":
            if (typeof json.relay !== "string") throw `there is no 'relay' field in json.\n${json}`;
            if (typeof json.client !== "string") throw `there is no 'client' field in json.\n${json}`;

            return createRelayAddress(json.relay, json.client);
        case "Service":
            if (typeof json.service !== "string") throw `there is no 'service' field in json.\n${json}`;
            return createServiceAddress(json.service);
        case "Peer":
            if (typeof json.peer !== "string") throw `there is no 'peer' field in json.\n${json}`;

            return createPeerAddress(json.peer);
        case undefined:
            throw `there is no 'type' field in json.\n${json}`;
        default:
            throw `'type' field should be only 'relay|service|peer' in json.\n${json}`;
    }
}

export function parseAddress(str: string): Address {
    let json = JSON.parse(str);

    return parseAddressObj(json);
}
