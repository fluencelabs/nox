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
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : new P(function (resolve) { resolve(result.value); }).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import { TendermintClient } from "../TendermintClient";
import { Engine } from "../Engine";
import { Signer } from "../Signer";
import { Client } from "../Client";
class CustomCommands {
    constructor(host, port) {
        let tm = new TendermintClient(host, port);
        let engine = new Engine(tm);
        // default signing key for now
        let signingKey = "TVAD4tNeMH2yJfkDZBSjrMJRbavmdc3/fGU2N2VAnxT3hAtSkX+Lrl4lN5OEsXjD7GGG7iEewSod472HudrkrA==";
        let signer = new Signer(signingKey);
        // `client001` is a default client for now
        let client = new Client("client001", signer);
        this.session = engine.genSession(client);
    }
    submit(command) {
        return __awaiter(this, void 0, void 0, function* () {
            console.log(`submit command: \"${command}\"`);
            let res = yield this.session.submitRaw(command);
            console.log(`the result is: \"${JSON.stringify(res)}\"`);
            return res;
        });
    }
}
const _global = (window /* browser */ || global /* node */);
_global.CustomCommands = CustomCommands;
