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
import {decode, encode} from "bs58"
import crypto from 'libp2p-crypto';
const ed25519 = crypto.keys.supportedKeys.ed25519;

export interface Trust {
    issuedFor: PeerId,
    expiresAt: number,
    signature: string,
    issuedAt: number
}

export function trustToString(trust: Trust): string {
    return `${encode(trust.issuedFor.pubKey.marshal())}\n${trust.signature}\n${trust.expiresAt}\n${trust.issuedAt}`
}

export async function trustFromString(issuedFor: string, signature: string, expiresAt: string, issuedAt: string): Promise<Trust> {
    let pubKey = ed25519.unmarshalEd25519PublicKey(decode(issuedFor));
    let peerId = await PeerId.createFromPubKey(pubKey.bytes);

    return {
        issuedFor: peerId,
        signature: signature,
        expiresAt: parseInt(expiresAt),
        issuedAt: parseInt(issuedAt)
    }
}

export async function createTrust(forPk: PeerId, issuedBy: PeerId, expiresAt: number, issuedAt: number): Promise<Trust> {
    let bytes = toSignMessage(forPk, expiresAt, issuedAt);

    let signature = await issuedBy.privKey.sign(Buffer.from(bytes));

    let signatureStr = encode(signature);

    return {
        issuedFor: forPk,
        expiresAt: expiresAt,
        signature: signatureStr,
        issuedAt: issuedAt
    };
}

function toSignMessage(pk: PeerId, expiresAt: number, issuedAt: number): Uint8Array {
    let bytes = new Uint8Array(48);
    let pkEncoded = pk.pubKey.marshal();

    bytes.set(pkEncoded, 0);
    bytes.set(numToArray(expiresAt), 32);
    bytes.set(numToArray(issuedAt), 40);

    return bytes
}

function numToArray(n: number): number[] {
    let byteArray = [0, 0, 0, 0, 0, 0, 0, 0];

    for (let index = 0; index < byteArray.length; index++) {
        let byte = n & 0xff;
        byteArray [index] = byte;
        n = (n - byte) / 256;
    }

    return byteArray;
}
