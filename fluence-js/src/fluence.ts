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
import {remove0x, secp256k1} from "./utils";
import {AppSession} from "./AppSession";

export {
    TendermintClient as TendermintClient,
    Engine as Engine,
    Session as Session,
    Result as Result,
    SessionConfig as SessionConfig,
    AppSession as AppSession
}

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
export async function connect(contract: string, appId: string, ethereumUrl?: string, privateKey?: Buffer | string): Promise<AppSession> {

    privateKey = convertPrivateKey(privateKey);

    let nodes: Node[] = await getAppNodes(contract, appId, ethereumUrl);
    let sessionId = Session.genSessionId();
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
    let tm = new TendermintClient(host, port, appId);
    let engine = new Engine(tm);

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
