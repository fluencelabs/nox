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
        throw Error("Last trust in chain should be same as 'issuedBy'.")
    }

    let trust = await createTrust(forPk, issuedBy, expiresAt, issuedAt);

    let chain = [...extendCert.chain];
    chain.push(trust);

    return {
        chain: chain
    }
}