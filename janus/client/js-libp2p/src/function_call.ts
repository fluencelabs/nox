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

import {Address, createPeerAddress, createRelayAddress, createServiceAddress, parseAddressObj} from "./address";
import * as PeerId from "peer-id";

/*
{ uuid: "123", target: { type: "service", service: "println"}, arguments: "privet omlet" }
 */
export interface FunctionCall {
    uuid: string,
    target: Address,
    reply_to?: Address,
    arguments: any,
    name?: string,
    action: "FunctionCall"
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

    let replyTo: Address;
    if (json.reply_to) replyTo = parseAddressObj(json.reply_to);

    if (!json.uuid) throw `there is no 'uuid' field in json.\n${str}`;
    if (!json.target) throw `there is no 'uuid' field in json.\n${str}`;

    let target = parseAddressObj(json.target);

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
export function makeRelayMsg(client: PeerId, relay: PeerId, msg: any, replyTo?: PeerId, name?: string): FunctionCall {
    let relayAddress = createRelayAddress(relay.toB58String(), client.toB58String());
    let reply_to;
    if (replyTo) reply_to = createPeerAddress(replyTo.toB58String());

    return makeFunctionCall(genUUID(), relayAddress, msg, reply_to, name);
}

/**
 * Message to peer
 */
export function makePeerMsg(client: PeerId, msg: any, replyTo?: PeerId, name?: string): FunctionCall {
    let peerAddress = createPeerAddress(client.toB58String());
    let reply_to;
    if (replyTo) reply_to = createPeerAddress(replyTo.toB58String());

    return makeFunctionCall(genUUID(), peerAddress, msg, reply_to, name);
}

/**
 * Message to call remote service
 */
export function makeCallMessage(functionId: string, args: any, replyTo?: PeerId, name?: string): FunctionCall {
    let target = createServiceAddress(functionId);
    let reply_to;
    if (replyTo) reply_to = createPeerAddress(replyTo.toB58String());

    return makeFunctionCall(genUUID(), target, args, reply_to, name);
}

/**
 * Message to register new service.
 */
export function makeRegisterMessage(serviceName: string, relayPeerId: PeerId, selfPeerId: PeerId): FunctionCall {
    let target = createServiceAddress("provide");
    let replyTo = createRelayAddress(relayPeerId.toB58String(), selfPeerId.toB58String());

    return makeFunctionCall(genUUID(), target, {service_id: serviceName}, replyTo, "provide service");
}

export function makeUnregisterMessage(serviceName: string, peerId: PeerId): FunctionCall {
    let target = createPeerAddress(peerId.toB58String());

    return makeFunctionCall(genUUID(), target, {key: serviceName}, undefined, "unregister");
}
