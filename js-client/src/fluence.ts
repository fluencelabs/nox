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

import { TendermintClient } from "./TendermintClient";
import { Engine } from "./Engine";
import { Signer } from "./Signer";
import { Client } from "./Client";
import { Session } from "./Session";
import { SessionConfig } from "./SessionConfig";
import {Empty, Result, Value, isValue} from "./Result";
import {getAppNodes, AppNode} from "fluence-monitoring"

export {
    TendermintClient as TendermintClient,
    Engine as Engine,
    Signer as Signer,
    Client as Client,
    Session as Session,
    Empty as Empty,
    Result as Result,
    Value as Value,
    isValue as isValue,
    SessionConfig as SessionConfig
}

// default signing key for now
let signingKey = "TVAD4tNeMH2yJfkDZBSjrMJRbavmdc3/fGU2N2VAnxT3hAtSkX+Lrl4lN5OEsXjD7GGG7iEewSod472HudrkrA==";
let signer = new Signer(signingKey);

// `client002` is a default client for now
let client = new Client("client002", signer);

export interface NodeSession {
    session: Session,
    appNode: AppNode
}

export interface AppSession {
    sessions: NodeSession[]
}

export async function createAppSessions(contract: string, appId: string): Promise<AppSession> {
    let appNodes: AppNode[] = await getAppNodes(contract, appId);
    let sessions: NodeSession[] = appNodes.map(an => {
        let session = createDefaultSession(an.node.ip_addr, an.port + 100);
        return {
            session: session,
            appNode: an
        }
    });
    return {
        sessions: sessions
    }
}

/**
 * Creates default session with default credentials.
 */
export function createDefaultSession(host: string, port: number) {
    let tm = new TendermintClient(host, port);

    let engine = new Engine(tm);

    return engine.genSession(client);
}
