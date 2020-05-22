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

import * as PeerId from "peer-id";
import {keys} from "libp2p-crypto";
import {Certificate, issueRoot} from "./certificate";

/**
 * Generate root certificate with one of the Fluence trusted key for one day.
 */
export async function nodeRootCert(issuedFor: PeerId): Promise<Certificate> {
    let seed = [46, 188, 245, 171, 145, 73, 40, 24, 52, 233, 215, 163, 54, 26, 31, 221, 159, 179, 126, 106, 27, 199, 189, 194, 80, 133, 235, 42, 42, 247, 80, 201];

    let privateK = await keys.generateKeyPairFromSeed("Ed25519", Uint8Array.from(seed), 256);
    let peerId = await PeerId.createFromPrivKey(privateK.bytes);

    let issuedAt = new Date();
    let expiresAt = new Date();
    expiresAt.setDate(new Date().getDate() + 1);

    return await issueRoot(peerId, issuedFor, expiresAt.getTime(), issuedAt.getTime());
}
