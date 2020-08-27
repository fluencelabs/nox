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
    Address, addressToString, parseAddress
} from "./address";
import { v4 as uuidv4 } from 'uuid';
import * as log from 'loglevel';

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

export function makeFunctionCall(uuid: string, target: Address, sender: Address, args: object, moduleId?: string, fname?: string, replyTo?: Address, name?: string): FunctionCall {

    return {
        uuid: uuid,
        target: target,
        reply_to: replyTo,
        sender: sender,
        "module": moduleId,
        fname: fname,
        arguments: args,
        name: name,
        action: "FunctionCall"
    }
}

export function parseFunctionCall(str: string): FunctionCall {
    let json = JSON.parse(str);
    log.debug(JSON.stringify(json, undefined, 2));

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
    return uuidv4();
}

/**
 * Message to provide new name.
 */
export async function makeProvideMessage(name: string, target: Address, sender: Address): Promise<FunctionCall> {
    return makeFunctionCall(genUUID(), target, sender, {name: name, address: addressToString(sender)}, "provide", undefined, sender, "provide service_id");
}

// TODO uncomment when this will be implemented in Fluence network
/*export function makeUnregisterMessage(serviceId: string, peerId: PeerId): FunctionCall {
    let target = createPeerAddress(peerId.toB58String());

    return makeFunctionCall(genUUID(), target, target, {key: serviceId}, undefined, "unregister");
}*/
