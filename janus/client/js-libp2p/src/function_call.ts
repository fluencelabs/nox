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
export function makeRelayCall(client: PeerId, relay: PeerId, msg: any, replyTo?: Address, name?: string): FunctionCall {
    let relayAddress = createRelayAddress(relay.toB58String(), client.toB58String());

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
export function makeRegisterMessage(serviceId: string, relayPeerId: PeerId, selfPeerId: PeerId): FunctionCall {
    let target = createServiceAddress("provide");
    let replyTo = createRelayAddress(relayPeerId.toB58String(), selfPeerId.toB58String());

    return makeFunctionCall(genUUID(), target, {service_id: serviceId}, replyTo, "provide service_id");
}

export function makeUnregisterMessage(serviceId: string, peerId: PeerId): FunctionCall {
    let target = createPeerAddress(peerId.toB58String());

    return makeFunctionCall(genUUID(), target, {key: serviceId}, undefined, "unregister");
}
