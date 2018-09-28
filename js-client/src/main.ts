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
import {Signer} from "./Signer";
import {Client} from "./Client";
import {Session} from "./Session";
import {SessionConfig} from "./SessionConfig";

export class Tester {

    private readonly engine: Engine;
    private readonly client: Client;

    constructor(host: string, port: number) {
        let tm = new TendermintClient(host, port);
        this.engine = new Engine(tm);
        console.log("dataengine created");

        // default signing key for now
        // TODO signing key in python client and in TS client provide a different key pair. Check this and fix.
        let signingKey = "TVAD4tNeMH2yJfkDZBSjrMJRbavmdc3/fGU2N2VAnxT3hAtSkX+Lrl4lN5OEsXjD7GGG7iEewSod472HudrkrA==";
        let signer = new Signer(signingKey);

        // `client002` is a default client for now
        this.client = new Client("client002", signer);
    }

    genSession(): Session {
        let config = new SessionConfig(2, 10, 120);
        return this.engine.genSession(this.client, config);
    }

    /**
     * Default function to invoke several commands to real-time cluster
     */
    async testIncrementAndMultiplyCluster() {

        let s = this.genSession();

        console.log("new session created");
        console.log("lets increment");

        await s.invokeRaw("inc()");
        s.invokeRaw("inc()");
        s.invokeRaw("inc()");
        s.invokeRaw("inc()");

        console.log("then get result");

        let resGet = await s.invoke("get");
        console.log(`result of incrementation is: ${JSON.stringify(resGet)}\n`);

        console.log("lets increment again");

        await s.invokeRaw("inc()");

        console.log("then get result again");

        let resGet2 = await s.invoke("get");
        console.log(`result of incrementation is: ${JSON.stringify(resGet2)}\n`);

        console.log("lets multiply two numbers, 72 and 114");
        let multiplyRes = await s.invoke("multiplier.mul", ["72", "114"]);
        console.log(`and the result is: ${JSON.stringify(multiplyRes)}`);

        console.log("lets multiply two numbers, 53 and 856");
        let multiplyRes2 = await s.invoke("multiplier.mul", ["53", "856"]);
        console.log(`and the result is: ${JSON.stringify(multiplyRes2)}`);
    }

    async testSync(count: number = 100) {

        let s = this.genSession();

        let init = await s.invoke("get");

        console.log("INIT = " + JSON.stringify(init));

        for(var i = 0; i < count; i++) {
            s.invoke("inc")
        }

        console.log("INVOKE RESULT");

        let pr = s.invoke("get");

        await s.sync();

        let result = await pr;
        console.log("RESULT = " + JSON.stringify(result));

    }

    async testCloseSession(count: number = 100) {
        let s = this.genSession();

        let init = await s.invoke("get");

        console.log("INIT = " + JSON.stringify(init));

        let threshold = count / 2 + count / 4;

        for(var i = 0; i < count; i++) {
            if (i === threshold) {
                s.invoke("random failed function");
            } else {
                s.invoke("inc")
            }
        }

        try {
            await s.sync();
        } catch (e) {
            console.log("ERROR = " + e)
        }


        console.log("INVOKE RESULT");

        let s2 = this.genSession();

        let pr = s2.invoke("get");



        let result = await pr;
        console.log("RESULT = " + JSON.stringify(result));
    }
}

// add testIncrementAndMultiplyCluster method to global scope
// TODO make the right imports
const _global = (window /* browser */ || global /* node */) as any;
_global.Tester = Tester;
