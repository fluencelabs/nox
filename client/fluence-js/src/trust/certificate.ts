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

import {createTrust, Trust, trustFromString, trustToString} from "./trust";
import * as PeerId from "peer-id";

const FORMAT = "11";
const VERSION = "1111";

// TODO verify certificate
// Chain of trusts started from self-signed root trust.
export interface Certificate {
    chain: Trust[]
}

export function certificateToString(cert: Certificate): string {
    let certStr = cert.chain.map(t => trustToString(t)).join("\n");
    return `${FORMAT}\n${VERSION}\n${certStr}`
}

export async function certificateFromString(str: string): Promise<Certificate> {
    let lines = str.split("\n");
    // last line could be empty
    if (!lines[lines.length - 1]) {
        lines.pop()
    }

    // TODO do match different formats and versions
    let _format = lines[0];
    let _version = lines[1];
    console.log("LENGTH: " + lines.length)

    // every trust is 4 lines, certificate lines number without format and version should be divided by 4
    if ((lines.length - 2) % 4 !== 0) {
        throw Error("Incorrect format of the certificate:\n" + str);
    }

    let chain: Trust[] = [];

    let i;
    for(i = 2; i < lines.length; i = i + 4) {
        chain.push(await trustFromString(lines[i], lines[i+1], lines[i+2], lines[i+3]))
    }

    return {chain};
}

// Creates new certificate with root trust (self-signed public key) from a key pair.
export async function issueRoot(issuedBy: PeerId,
                                forPk: PeerId,
                                expiresAt: number,
                                issuedAt: number,
): Promise<Certificate> {
    if (expiresAt < issuedAt) {
        throw Error("Expiration time should be greater then issued time.")
    }

    let maxDate = new Date(158981172690500).getTime();

    let rootTrust = await createTrust(issuedBy, issuedBy, maxDate, issuedAt);
    let trust = await createTrust(forPk, issuedBy, expiresAt, issuedAt);
    let chain = [rootTrust, trust];

    return {
        chain: chain
    }
}

// Adds a new trust into chain of trust in certificate.
export async function issue(issuedBy: PeerId,
                            forPk: PeerId,
                            extendCert: Certificate,
                            expiresAt: number,
                            issuedAt: number): Promise<Certificate> {
    if (expiresAt < issuedAt) {
        throw Error("Expiration time should be greater then issued time.")
    }

    let lastTrust = extendCert.chain[extendCert.chain.length - 1];

    if (lastTrust.issuedFor !== issuedBy) {
        throw Error("`issuedFor` should be equal to `issuedBy` in the last trust of the chain.")
    }

    let trust = await createTrust(forPk, issuedBy, expiresAt, issuedAt);
    let chain = [...extendCert.chain];
    chain.push(trust);

    return {
        chain: chain
    }
}
