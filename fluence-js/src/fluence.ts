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

import {TendermintClient} from "./TendermintClient";
import {Engine} from "./Engine";
import {Session} from "./Session";
import {SessionConfig} from "./SessionConfig";
import {Result} from "./Result";
import {getAppNodes, Node} from "./contract"
import {remove0x, secp256k1, parseHost, genSessionId} from "./utils";
import {AppSession} from "./AppSession";
import * as randomstring from "randomstring";
import {toByteArray} from "base64-js";
import {WebsocketSession} from "./WebsocketSession";

export {
    TendermintClient as TendermintClient,
    Engine as Engine,
    Session as Session,
    Result as Result,
    SessionConfig as SessionConfig,
    AppSession as AppSession
}

let defaultContract = "0xe01690f60E08207Fa29F9ef98fA35e7fB7A12A96";

// A session with a worker with info about a worker
export interface WorkerSession {
    session: Session,
    node: Node
}

function convertPrivateKey(privateKey?: Buffer | string): undefined | Buffer {
    if (privateKey != undefined && typeof privateKey == 'string') {
        privateKey = Buffer.from(remove0x(privateKey), "hex");
    }

    if (privateKey != undefined) {
        if (!secp256k1.privateKeyVerify(privateKey)) {
            throw Error("Private key is invalid");
        }
    }

    return privateKey
}

/**
 * Creates a connection with an app (all nodes hosting an app)
 * @param contract Contract address to read app's nodes list from
 * @param appId Target app
 * @param ethereumUrl Optional ethereum node url. Connect via Metamask if `ethereumlUrl` is undefined
 * @param privateKey Optional private key to sign requests. Signature is concatenated to the request payload.
 */
export async function connect(appId: string, contract?: string, ethereumUrl?: string, privateKey?: Buffer | string): Promise<AppSession> {

    privateKey = convertPrivateKey(privateKey);

    contract = contract ? contract : defaultContract;

    let nodes: Node[] = await getAppNodes(contract, appId, ethereumUrl);
    let sessionId = genSessionId();
    let sessions: WorkerSession[] = nodes.map(node => {
        let session = sessionConnect(node.ip_addr, node.api_port, appId, sessionId);
        return {
            session: session,
            node: node
        }
    });

    return new AppSession(sessionId, appId, sessions, privateKey);
}

/**
 * Creates connection to one node.
 */
function sessionConnect(host: string, port: number, appId: string, sessionId?: string) {
    const { protocol, hostname } = parseHost(host);
    const tm = new TendermintClient(hostname, port, appId, protocol);
    const engine = new Engine(tm);

    if (sessionId == undefined) {
        return engine.genSession();
    } else {
        return engine.createSession(new SessionConfig(), sessionId);
    }
}

/**
 * Creates app session with one node.
 */
export function directConnect(host: string, port: number, appId: string, sessionId?: string, privateKey?: Buffer | string) {
    let session = sessionConnect(host, port, appId, sessionId);
    privateKey = convertPrivateKey(privateKey);

    let sessions: WorkerSession[] = [
        {
            session: session,
            // @ts-ignore
            node: undefined
        }
    ];

    return new AppSession(session.session, appId, sessions, privateKey);
}

export async function websocket(appId: string, contract?: string, ethereumUrl?: string, privateKey?: Buffer | string) {
    privateKey = convertPrivateKey(privateKey);
    contract = contract ? contract : defaultContract;
    let nodes: Node[] = await getAppNodes(contract, appId, ethereumUrl);

    return new WebsocketSession(appId, nodes, privateKey);
}

export function testWebsocket() {
    let socket = new WebSocket("ws://46.101.151.125:25000/apps/96/ws");

    let req = function(r: string) {

        let rnd = randomstring.generate(12);
        let path = `${rnd}/0`;

        let tx = `${path}\n${r}`;

        return {
            tx: tx,
            request_id: "ididid",
            type: "tx_wait_request"
        };
    };

    let reqSub = function(r: string) {

        return {
            tx: r,
            request_id: "ididid",
            subscription_id: "ididid",
            type: "subscribe_request"
        };
    };


    /*
    {"request_id":"ididid","data":"{\n  \"jsonrpc\": \"2.0\",\n  \"id\": \"dontcare\",\n  \"result\": {\n    \"code\": 1,\n    \"data\": \"43616E6E6F74207061727365207472616E73616374696F6E20686561646572\",\n    \"log\": \"\",\n    \"hash\": \"E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855\"\n  }\n}","type":"tx_response"}
     */

    socket.onmessage = function (event) {
        // console.log(event.data);
        let parsed = JSON.parse(JSON.parse(event.data).data).result.response;
        // console.log(parsed);

        let result = new Result(toByteArray(parsed.value));
        console.log("result: " + result.asString())

    };

    // create table employee(empid integer,name varchar(20),title varchar(10));
    // insert into employee values(101,'John Smith','CEO');
    // select * from employee;

    socket.onopen = function (e) {
        let req1 = req("insert into employee values(101,'John Smith','CEO');");
        let sub1 = reqSub("select * from employee;");

        socket.send(JSON.stringify(req1));
        socket.send(JSON.stringify(sub1));
    };
}
