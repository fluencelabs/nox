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

'use strict';

import {ipfsAdd, ipfsGet} from "./ipfs";
import {JanusClient} from "./janus";
import {FunctionCall, genUUID, makeFunctionCall} from "./function_call";

interface Server {
    peer: string,
    ip: string,
    port: number
}

function server(peer: string, ip: string, port: number): Server {
    return {
        peer: peer,
        ip: ip,
        port: port
    }
}

const servers = [
    server("QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz", "104.248.25.59", 9001),
    server("QmVzDnaPYN12QAYLDbGzvMgso7gbRD9FQqRvGZBfeKDSqW", "104.248.25.59", 9002),
    server("QmX6yYZd4iLW7YpmZz4waLrtb5Y9f5v3PPGEmNGh9k3iW2", "104.248.25.59", 9990)
];

// Shows how to register and call new service in Fluence network
async function testCalculator() {
    // connect to two different nodes
    let con1 = await JanusClient.connect("QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz", "104.248.25.59", 9001);
    let con2 = await JanusClient.connect("QmVzDnaPYN12QAYLDbGzvMgso7gbRD9FQqRvGZBfeKDSqW", "104.248.25.59", 9002);

    // service name that we will register with one connection and call with another
    let serviceName = "sum-calculator";

    // register service that will add two numbers and send a response with calculation result
    await con1.registerService(serviceName, async (req) => {
        console.log("message received");
        console.log(req);

        console.log("send response");

        let response = makeFunctionCall(genUUID(), req.reply_to, {msgId: req.arguments.msgId, result: req.arguments.one + req.arguments.two});
        console.log(response);

        await con1.sendFunctionCall(response);
    });


    // msgId is to identify response
    let msgId = "calculate-it-for-me";

    let req = {one: 12, two: 23, msgId: msgId};

    // send call to `sum-calculator` service with two numbers
    await con2.sendServiceCall(serviceName, req, "calculator request");

    let resultPromise: Promise<number> = new Promise((resolve, reject) => {
        // subsribe for responses, to handle response
        con2.subscribe((call: FunctionCall) => {
            if (call.arguments.msgId && call.arguments.msgId === msgId) {
                console.log("response received!");

                resolve(call.arguments.result);
                return true;
            }
            return false;
        });
    });

    let result = await resultPromise;
    console.log(`calculation result is: ${result}`);
}

async function checkIPFS() {
    // let multiaddr = '/ip4/127.0.0.1/tcp/5001';
    let multiaddr = '/dns4/ipfs1.fluence.one/tcp/5001';

    let f: any = await ipfsAdd(multiaddr, new Uint8Array([1,2,3,4]));
    let res = await ipfsGet(multiaddr, f.path);

    console.log(res);
}