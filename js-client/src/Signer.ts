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

import * as elliptic from "elliptic";
import {utils} from "elliptic";
import * as base64js from "base64-js";
import sha256 from "fast-sha256";

/**
 * Key with the ability to make signatures from a string.
 */
export class Signer {

    private readonly key: any;

    constructor(signingKey: string) {
        let ec = new elliptic.eddsa('ed25519');

        let keyS = utils.toHex(base64js.toByteArray(signingKey));

        this.key = ec.keyFromSecret(keyS);
    }

    /**
     * Signs string and return signature in base64
     * @param str string to sign
     */
    sign(str: string): string {

        let signBytes = utils.toArray(str);

        let hashed = sha256(signBytes);

        let signature = this.key.sign(hashed);

        return base64js.fromByteArray(signature.toBytes())
    }
}
