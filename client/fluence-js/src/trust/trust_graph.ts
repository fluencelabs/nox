/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

import {FluenceClient} from "../fluence_client";
import {Certificate, certificateFromString, certificateToString} from "./certificate";
import {genUUID} from "../function_call";

export class TrustGraph {

    client: FluenceClient;

    constructor(client: FluenceClient) {
        this.client = client;
    }

    async addCerts(peerId: string, certs: Certificate[]) {
        let certsStr = [];
        for (let cert of certs) {
            certsStr.push(await certificateToString(cert));
        }

        await this.client.sendServiceCall("add_certificates", {
            certificates: certsStr,
            msg_id: genUUID(),
            peer_id: peerId
        });
    }

    async getCerts(peerId: string): Promise<Certificate[]> {
        let msgId = genUUID();
        let resp = await this.client.sendServiceCallWaitResponse("certificates", {
            msg_id: msgId,
            peer_id: peerId
        }, (args) => args.msg_id && args.msg_id === msgId)

        let certificatesRaw = resp.certificates

        console.log("parsed:")
        console.dir(resp)

        if (!(certificatesRaw && Array.isArray(certificatesRaw))) {
            console.log(Array.isArray(certificatesRaw))
            throw Error("Unexpected. Certificates should be presented in the response as an array.")
        }

        let certs = [];
        for (let cert of certificatesRaw) {
            certs.push(await certificateFromString(cert))
        }

        return certs;
    }
}