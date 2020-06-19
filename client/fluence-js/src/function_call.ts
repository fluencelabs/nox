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

import {
    createPeerAddress,
    createRelayAddress,
    createServiceAddress,
    Address, addressToString, parseAddress
} from "./address";
import * as PeerId from "peer-id";

export interface FunctionCall {
    uuid: string,
    target: Address,
    reply_to?: Address,
    sender: Address,
    "module"?: string,
    fname?: string,
    arguments: any,
    name?: string,
    action: "FunctionCall"
}

export function callToString(call: FunctionCall) {
    let obj: any = {...call};

    if (obj.reply_to) {
        obj.reply_to = addressToString(obj.reply_to);
    }

    obj.target = addressToString(obj.target);
    obj.sender = addressToString(obj.sender);

    return JSON.stringify(obj)
}

export function makeFunctionCall(uuid: string, target: Address, sender: Address, args: object, moduleF?: string, fname?: string, replyTo?: Address, name?: string): FunctionCall {

    return {
        uuid: uuid,
        target: target,
        reply_to: replyTo,
        sender: sender,
        "module": moduleF,
        fname: fname,
        arguments: args,
        name: name,
        action: "FunctionCall"
    }
}

export function parseFunctionCall(str: string): FunctionCall {
    let json = JSON.parse(str);
    console.log(JSON.stringify(json, undefined, 2));

    let replyTo: Address;
    if (json.reply_to) replyTo = parseAddress(json.reply_to);

    if (!json.uuid) throw Error(`there is no 'uuid' field in json.\n${str}`);
    if (!json.target) throw Error(`there is no 'uuid' field in json.\n${str}`);
    if (!json.sender) throw Error(`there is no 'sender' field in json.\n${str}`);

    let target = parseAddress(json.target);
    let sender = parseAddress(json.sender);

    return {
        uuid: json.uuid,
        target: target,
        reply_to: replyTo,
        sender: sender,
        arguments: json.arguments,
        "module": json.module,
        fname: json.fname,
        name: json.name,
        action: "FunctionCall"
    }
}

export function genUUID() {
    let date = new Date();
    return date.toISOString()
}

/**
 * Message to peer through relay
 */
export async function makeRelayCall(client: PeerId, relay: PeerId, msg: any, sender: Address, replyTo?: Address, name?: string): Promise<FunctionCall> {
    let relayAddress = await createRelayAddress(relay.toB58String(), client, false);

    return makeFunctionCall(genUUID(), relayAddress, sender, msg, undefined, undefined, replyTo, name);
}

/**
 * Message to peer
 */
export function makePeerCall(client: PeerId, msg: any, sender: Address, replyTo?: Address, name?: string): FunctionCall {
    let peerAddress = createPeerAddress(client.toB58String());

    return makeFunctionCall(genUUID(), peerAddress, sender, msg, undefined, undefined, replyTo, name);
}

/**
 * Message to call remote service_id
 */
export function makeCall(functionId: string, target: Address, args: any, sender: Address, replyTo?: Address, name?: string): FunctionCall {


    return makeFunctionCall(genUUID(), target, sender, args, functionId, undefined, replyTo, name);
}

/**
 * Message to register new service_id.
 */
export async function makeRegisterMessage(serviceId: string, target: Address, sender: Address): Promise<FunctionCall> {
    return makeFunctionCall(genUUID(), target, sender, {service_id: serviceId}, "provide", undefined, sender, "provide service_id");
}

// TODO uncomment when this will be implemented in Fluence network
/*export function makeUnregisterMessage(serviceId: string, peerId: PeerId): FunctionCall {
    let target = createPeerAddress(peerId.toB58String());

    return makeFunctionCall(genUUID(), target, target, {key: serviceId}, undefined, "unregister");
}*/
