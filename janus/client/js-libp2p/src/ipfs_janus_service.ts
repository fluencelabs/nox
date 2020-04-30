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

import {JanusClient} from "./janus";
import {FunctionCall, genUUID, makeFunctionCall} from "./function_call";
import {calcHash, ipfsAdd} from "./ipfs";
import {createServiceAddress} from "./address";

export class IpfsShowService {

    connection: JanusClient;
    file: Uint8Array;
    private hash: string;
    multiaddrService: string;
    uploadHook: (hash: string) => void;

    /**
     *
     * @param connection janus connection
     * @param file bytes in base64
     * @param multiaddrService service name to get IPFS node multiaddr
     * @param uploadHook hook is called when a remote client calls this service
     */
    constructor(connection: JanusClient, file: Uint8Array, multiaddrService: string, uploadHook: (hash: string) => void) {
        this.connection = connection;
        this.file = file;
        this.multiaddrService = multiaddrService;
        this.uploadHook = uploadHook;
    }

    async initHash(): Promise<void> {
        if (!this.hash) {
            let buf = Buffer.from(this.file);
            this.hash = await calcHash(buf);
        }
    }

    async getHash(): Promise<string> {
        await this.initHash();
        return this.hash;
    }

    async getName(): Promise<string> {
        if (this.hash) {
            console.log("without calculation");
            return "IPFS.get_" + this.hash;
        } else {
            console.log("with calculation");
            await this.initHash();
            return "IPFS.get_" + this.hash;
        }
    }

    service(log: (hash: string, msg: string) => void): (call: FunctionCall) => Promise<void> {

        console.log("HASH: " + this.hash);

        let thisService = this;

        return async (call: FunctionCall) => {

            if (!thisService.hash) {
                throw "IpfsService: calculate hash with `initHash` method before use"
            }

            let sender;
            if (call.reply_to && call.reply_to.type === "Relay") {
                sender = call.reply_to.client
            }

            let serviceName = await thisService.getName();

            log(thisService.hash, `Received function call ${serviceName} from ${JSON.stringify(sender)}`);

            let uuid = genUUID();

            let msgIdIpfsAddr = uuid + thisService.hash;

            let ipfsServiceResponse: Promise<string> = new Promise((resolve, reject) => {
                thisService.connection.subscribe((call: FunctionCall) => {
                    if (call.arguments.msg_id && call.arguments.msg_id === msgIdIpfsAddr) {
                        if (call.arguments.multiaddr) {
                            log(thisService.hash, `Function ${thisService.multiaddrService} call result: ${call.arguments.multiaddr}`);
                            resolve(call.arguments.multiaddr as string);
                            return true;
                        } else {
                            reject("IpfsService: `msgId` equals but there is no `multiaddr`, delete subscription but can't do anything further");
                            return true;
                        }
                    }
                })
            });

            log(thisService.hash, `Searching for available IPFS node`);

            log(thisService.hash, `Sending ${thisService.multiaddrService} call`);



            let ipfsAddrService = createServiceAddress(thisService.multiaddrService);
            let ipfsAddrCall: FunctionCall = makeFunctionCall(uuid, ipfsAddrService, {msg_id: msgIdIpfsAddr}, thisService.connection.replyToAddress, thisService.multiaddrService);

            console.log("IpfsService: send call to multiaddr:");
            console.log(ipfsAddrCall);
            console.log("");

            await thisService.connection.sendFunctionCall(ipfsAddrCall);

            let multiaddr = await ipfsServiceResponse;

            console.log("IpfsService: multiaddr received: " + multiaddr);

            log(thisService.hash, "Uploading file directly to IPFS node");

            await ipfsAdd(multiaddr, thisService.file);
            console.log("IpfsService: FILE UPLOADED");

            let response = makeFunctionCall(genUUID(), call.reply_to, { multiaddr: multiaddr, msg_id: call.arguments.msg_id}, undefined, "response to " + serviceName);

            await thisService.connection.sendFunctionCall(response);

            this.uploadHook(this.hash);

            log(thisService.hash, `Sending result on function ${serviceName} to ${sender}`);

            /*
            to check this part:
            - browser1: create this service
            - browser2: create service Multiaddr
            - browser3: create peer and call first service
            - browser1: check that correct multiaddr returns
             */
        }
    }
}
