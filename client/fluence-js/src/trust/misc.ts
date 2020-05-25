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
