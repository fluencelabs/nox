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

export class IpfsMultiaddrService {

    connection: JanusClient;
    multiaddr: string;

    constructor(connection: JanusClient, multiaddr: string) {
        this.connection = connection;
        this.multiaddr = multiaddr;
    }

    service(): (call: FunctionCall) => Promise<void> {

        let _this = this;

        return async (call: FunctionCall) => {
            let uuid = genUUID();

            console.log("Multiaddr Service: call received:");
            console.log(call);

            if (call.reply_to) {
                let response: FunctionCall = makeFunctionCall(uuid, call.reply_to, { multiaddr: _this.multiaddr, msg_id: call.arguments.msg_id }, undefined, "IPFS.multiaddr");

                console.log("Multiaddr Service: send response:");
                console.log(response);
                console.log("");

                _this.connection.sendFunctionCall(response);
            } else {
                console.log(`MultiaddrService: no 'reply_to' in call: ${JSON.stringify(call)}`)
            }
        };


    }
}
