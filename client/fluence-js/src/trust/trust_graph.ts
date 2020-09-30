/*
 * Copyright 2020 Fluence Labs Limited
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

import {FluenceClient} from "../fluenceClient";
import {Certificate, certificateFromString, certificateToString} from "./certificate";
import * as log from 'loglevel';

// TODO update after 'aquamarine' implemented
// The client to interact with the Fluence trust graph API
export class TrustGraph {

    client: FluenceClient;

    constructor(client: FluenceClient) {
        this.client = client;
    }

    // Publish certificate to Fluence network. It will be published in Kademlia neighbourhood by `peerId` key.
    async publishCertificates(peerId: string, certs: Certificate[]) {
        let certsStr = [];
        for (let cert of certs) {
            certsStr.push(await certificateToString(cert));
        }

        /*let response = await this.client.callPeer("add_certificates", {
            certificates: certsStr,
            peer_id: peerId
        });*/
        let response: any = {};

        if (response.reason) {
            throw Error(response.reason)
        } else if (response.status) {
            return response.status
        } else {
            throw Error(`Unexpected response: ${response}. Should be 'status' field for a success response or 'reason' field for an error.`)
        }
    }

    // Get certificates that stores in Kademlia neighbourhood by `peerId` key.
    async getCertificates(peerId: string): Promise<Certificate[]> {
        let resp: any = {};
        /*let resp = await this.client.callPeer("certificates", {
            peer_id: peerId
        });*/

        let certificatesRaw = resp.certificates

        if (!(certificatesRaw && Array.isArray(certificatesRaw))) {
            log.error(Array.isArray(certificatesRaw))
            throw Error("Unexpected. Certificates should be presented in the response as an array.")
        }

        let certs = [];
        for (let cert of certificatesRaw) {
            certs.push(await certificateFromString(cert))
        }

        return certs;
    }
}
