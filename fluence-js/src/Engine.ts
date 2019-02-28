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
import {SessionConfig} from "./SessionConfig";

/**
 * An entry point for the client.
 */
export class Engine {
    readonly tm: TendermintClient;

    /**
     * @param _tm transport client for interaction with the cluster
     */
    constructor(_tm: TendermintClient) {
        this.tm = _tm;
    }

    /**
     * Creates a new session with a random identifier with the real-time cluster.
     * @param config parameters that regulate the session
     */
    genSession(config: SessionConfig = new SessionConfig()): Session {
        return new Session(this.tm, config)
    }

    /**
     * Creates new session with the real-time cluster.
     * @param config parameters that regulate the session
     * @param sessionId session identifier
     */
    createSession(config: SessionConfig = new SessionConfig(), sessionId: string): Session {
        return new Session(this.tm, config, sessionId)
    }
}
