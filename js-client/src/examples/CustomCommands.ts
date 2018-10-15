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

import {Session} from "../Session";
import {TendermintClient} from "../TendermintClient";
import {Engine} from "../Engine";
import {Signer} from "../Signer";
import {Client} from "../Client";
import {isValue} from "../Result";
import {fromHex} from "../utils";

class CustomCommands {

    private session: Session;

    constructor(host: string, port: number) {
        let tm = new TendermintClient(host, port);

        let engine = new Engine(tm);

        // default signing key for now
        let signingKey = "TVAD4tNeMH2yJfkDZBSjrMJRbavmdc3/fGU2N2VAnxT3hAtSkX+Lrl4lN5OEsXjD7GGG7iEewSod472HudrkrA==";
        let signer = new Signer(signingKey);

        // `client002` is a default client for now
        let client = new Client("client002", signer);

        this.session = engine.genSession(client);
    }

    async submit(command: string, arg: string) {
        let res = await this.session.invoke(command, arg).result();
        if (isValue(res)) {
            let strResult = fromHex(res.hex());
            console.log(`the result is:\n ${strResult}`);
        } else {
            console.log(`the result is empty`);
        }
        return res;
    }
}

const _global = (window /* browser */ || global /* node */) as any;
_global.CustomCommands = CustomCommands;