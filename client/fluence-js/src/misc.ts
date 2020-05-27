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
import {keys} from "libp2p-crypto";

/**
 * @param seed 32 bytes
 */
export async function seedToPeerId(seed: string): Promise<PeerId> {
    let seedArr = decode(seed);

    let privateK = await keys.generateKeyPairFromSeed("Ed25519", Uint8Array.from(seedArr), 256);
    return await PeerId.createFromPrivKey(privateK.bytes);
}

export function peerIdToSeed(peerId: PeerId): string {
    let seedBuf = peerId.privKey.marshal().subarray(0, 32)
    return encode(seedBuf)
}
