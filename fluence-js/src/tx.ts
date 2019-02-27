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

import {Signer} from "./Signer";
import {Client} from "./Client";
import * as base64js from "base64-js";

/**
 * Generates execution command's transaction in hex representation for the real-time cluster.
 *
 * @param client identity of clientId and signer
 * @param session identity of session
 * @param counter transaction counter for ordering
 * @param payload command with arguments
 */
export function genTxBase64(client: Client, session: string, counter: number, payload: string): string {
    let tx = new TxEncoder(client.id, session, counter, payload).createJson(client.signer);
    return jsonToBase64(tx)
}

/**
 * Makes hex string from json object that will be correct to the real-time cluster.
 * @param json transaction with command, metainformation and signature
 */
function jsonToBase64(json: TxJson): string {
    return base64js.fromByteArray(Buffer.from(JSON.stringify(json)));
}

/**
 * Class to aggregate information and create signed transaction json for the real-time cluster.
 */
class TxEncoder {
    private readonly tx: Tx;

    constructor(client: string, session: string, counter: number, payload: string) {
        this.tx = {
            header: {
                client: client,
                session: session,
                order: counter
            },
            payload: payload,
            timestamp: +new Date
        };
    }

    /**
     * Creates a correct json representation for the cluster
     * @param signer
     */
    createJson(signer: Signer): TxJson {

        let header = this.tx.header;
        let str = `${header.client}-${header.session}-${header.order}-${this.tx.payload}`;

        let signature = signer.sign(str);

        return {
            tx: this.tx,
            signature: signature
        };
    }
}

/**
 * The header of the transaction.
 */
interface Header {
    client: string;
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

/**
 * The transaction with signature.
 */
export interface TxJson {
    tx: Tx
    signature: string
}
