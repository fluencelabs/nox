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

import {utils} from 'elliptic'
import {wrapInQuotes} from "./utils";
import {Signer} from "./Signer";
import {Client} from "./Client";

/**
 * Generate execution command's transaction in hex representation for tendermint cluster.
 *
 * @param client identity of clientId and signer
 * @param session identity of session
 * @param counter transaction counter for ordering
 * @param payload command with arguments
 */
export function genTxHex(client: Client, session: string, counter: number, payload: string): string {
    let tx = new TxCl(client.id, session, counter, payload).createJson(client.signer);
    return jsonToHex(tx)
}

/**
 * Makes string gtom object that correct to tendermint cluster
 * @param json transaction with command, meta information and signature
 */
function jsonToHex(json: TxJson): string {
    return wrapInQuotes(utils.toHex(Buffer.from(JSON.stringify(json))).toUpperCase())
}

class TxCl {
    tx: Tx;

    constructor(client: string, session: string, counter: number, payload: string) {
        this.tx = {
            header: {
                client: client,
                session: session,
                order: counter
            },
            payload: payload
        };
    }

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

interface Header {
    client: string;
    session: string;
    order: number;
}

interface Tx {
    header: Header;
    payload: string
}

export interface TxJson {
    tx: Tx
    signature: string
}