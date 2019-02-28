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

import * as base64js from "base64-js";

/**
 * Generates execution command's transaction in hex representation for the real-time cluster.
 *
 * @param client identity of clientId and signer
 * @param session identity of session
 * @param counter transaction counter for ordering
 * @param payload command with arguments
 */
export function genTxBase64(session: string, counter: number, payload: string): string {
    let tx = new TxEncoder(session, counter, payload).tx;
    return jsonToBase64(tx)
}

/**
 * Makes hex string from json object that will be correct to the real-time cluster.
 * @param json transaction with command, metainformation and signature
 */
function jsonToBase64(json: Tx): string {
    return base64js.fromByteArray(Buffer.from(JSON.stringify(json)));
}

/**
 * Class to aggregate information and create signed transaction json for the real-time cluster.
 */
class TxEncoder {
    public readonly tx: Tx;

    constructor(session: string, counter: number, payload: string) {
        this.tx = {
            header: {
                session: session,
                order: counter
            },
            payload: payload,
            timestamp: +new Date
        };
    }
}

/**
 * The header of the transaction.
 */
interface Header {
    session: string;
    order: number;
}

/**
 * Json representation of the transaction.
 */
interface Tx {
    header: Header;
    payload: string;
    timestamp: number;
}
