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
import {Session} from "./Session";
import {Client} from "./Client";

/**
 * Entrypoint for client.
 */
export class Engine {
    tm: TendermintClient;

    /**
     * @param _tm transport client for interaction with tendermint
     */
    constructor(_tm: TendermintClient) {
        this.tm = _tm;
    }

    /**
     * Creates new session with random identifier with tendermint cluster.
     * @param client identifier of client and signer
     */
    genSession(client: Client): Session {
        return new Session(this.tm, client)
    }

    /**
     * Creates new session with tendermint cluster.
     * @param client identifier of client and signer
     * @param sessionId session identifier
     */
    createSession(client: Client, sessionId: string): Session {
        return new Session(this.tm, client, sessionId)
    }
}
