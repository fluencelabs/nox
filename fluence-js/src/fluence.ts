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
import { Session } from "./Session";
import { SessionConfig } from "./SessionConfig";
import {Result} from "./Result";
import {getAppNodes, Node} from "fluence-monitoring"
import { ResultPromise } from "./ResultAwait";

export {
    TendermintClient as TendermintClient,
    Engine as Engine,
    Session as Session,
    Result as Result,
    SessionConfig as SessionConfig
}

// A session with a worker with info about a worker
export interface WorkerSession {
    session: Session,
    node: Node
}

// All sessions with workers from an app
export interface AppSession {
    appId: string,
    workerSessions: WorkerSession[],
    invoke(payload: string): ResultPromise
}

/*
 * Creates connection with an app (all nodes related to an app in Fluence contract)
 */
export async function connect(contract: string, appId: string, ethereumUrl?: string): Promise<AppSession> {
    let nodes: Node[] = await getAppNodes(contract, appId, ethereumUrl);
    let sessions: WorkerSession[] = nodes.map(node => {
        let session = directConnect(node.ip_addr, node.api_port, appId);
        return {
            session: session,
            node: node
        }
    });

    // randomly selects worker and calls `invoke` on that worker
    function invoke(payload: string): ResultPromise {
        function getRandom(floor:number, ceiling:number) {
            return Math.floor(Math.random() * (ceiling - floor + 1)) + floor;
        }

        const randomChoiceIndex = getRandom(0, sessions.length - 1);
        let session = sessions[randomChoiceIndex].session;
        return session.invoke(payload);
    }

    return {
        appId: appId,
        workerSessions: sessions,
        invoke: invoke
    }
}

/**
 * Creates direct connection to one node.
 */
export function directConnect(host: string, port: number, appId: string) {
    let tm = new TendermintClient(host, port, appId);

    let engine = new Engine(tm);

    return engine.genSession();
}
