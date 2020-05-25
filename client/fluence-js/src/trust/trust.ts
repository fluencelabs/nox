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

import * as PeerId from "peer-id";
import {decode, encode} from "bs58"
import crypto from 'libp2p-crypto';
const ed25519 = crypto.keys.supportedKeys.ed25519;

// One element in chain of trust in a certificate.
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
