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

import {TendermintClient} from "../TendermintClient";
import {Engine} from "../Engine";
import {Signer} from "../Signer";
import {Client} from "../Client";
import {Session} from "../Session";
import {isValue} from "../Result";

class IncrementAndMultiply {

    // a session is needed for ordered transactions, you can use multiple sessions if needed
    private session: Session;

    constructor(host: string, port: number) {
        // there is initializing RPC to tendermint cluster
        let tm = new TendermintClient(host, port);

        // creates engine that can start new sessions
        let engine = new Engine(tm);

        // default signing key for now
        // signing key can be generated or received after some authorize processes
        let signingKey = "TVAD4tNeMH2yJfkDZBSjrMJRbavmdc3/fGU2N2VAnxT3hAtSkX+Lrl4lN5OEsXjD7GGG7iEewSod472HudrkrA==";

        // creates signer that can sign messages
        let signer = new Signer(signingKey);

        // `client002` is a default client for now
        // creates client with id and signer
        let client = new Client("client002", signer);

        // generates the random session. If you want to generate session on your own - use createSession(client, "some-id")
        this.session = engine.genSession(client);
    }

    // uses the session to submit commands you want to
    async incrementCounter() {
        console.log("invoke counter");

        let res = await this.session.invoke().result();
        if (isValue(res)) {
            console.log(`get result is: ${JSON.stringify(res.asInt())}`);
        }
        return res
    }

    async multiply(first: number, second: number) {
        let res = await this.session.invoke(`[${first.toString()}, ${second.toString()}]`)
            .result();
        console.log(`multiply result is: ${JSON.stringify(res)}`);
        return res;
    }
}

const _global = (window /* browser */ || global /* node */) as any;
_global.IncrementAndMultiply = IncrementAndMultiply;