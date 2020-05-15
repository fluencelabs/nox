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

    return JSON.stringify(obj)
}

export function makeFunctionCall(uuid: string, target: Address, args: object, replyTo?: Address, name?: string): FunctionCall {

    return {
        uuid: uuid,
        target: target,
        reply_to: replyTo,
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

    let target = parseAddress(json.target);

    return {
        uuid: json.uuid,
        target: target,
        reply_to: replyTo,
        arguments: json.arguments,
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
export async function makeRelayCall(client: PeerId, relay: PeerId, msg: any, replyTo?: Address, name?: string): Promise<FunctionCall> {
    let relayAddress = await createRelayAddress(relay.toB58String(), client, false);

    return makeFunctionCall(genUUID(), relayAddress, msg, replyTo, name);
}

/**
 * Message to peer
 */
export function makePeerCall(client: PeerId, msg: any, replyTo?: Address, name?: string): FunctionCall {
    let peerAddress = createPeerAddress(client.toB58String());

    return makeFunctionCall(genUUID(), peerAddress, msg, replyTo, name);
}

/**
 * Message to call remote service_id
 */
export function makeCall(functionId: string, args: any, replyTo?: Address, name?: string): FunctionCall {
    let target = createServiceAddress(functionId);

    return makeFunctionCall(genUUID(), target, args, replyTo, name);
}

/**
 * Message to register new service_id.
 */
export async function makeRegisterMessage(serviceId: string, relayPeerId: PeerId, selfPeerId: PeerId): Promise<FunctionCall> {
    let target = createServiceAddress("provide");
    let replyTo = await createRelayAddress(relayPeerId.toB58String(), selfPeerId, true);

    return makeFunctionCall(genUUID(), target, {service_id: serviceId}, replyTo, "provide service_id");
}

export function makeUnregisterMessage(serviceId: string, peerId: PeerId): FunctionCall {
    let target = createPeerAddress(peerId.toB58String());

    return makeFunctionCall(genUUID(), target, {key: serviceId}, undefined, "unregister");
}
