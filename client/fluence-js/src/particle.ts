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

import { v4 as uuidv4 } from 'uuid';
import PeerId from "peer-id";
import {encode} from "bs58";

export interface Particle {
    id: string,
    init_peer_id: string,
    timestamp: number,
    ttl: number,
    script: string,
    // sign upper fields
    signature: string,
    data: object,
    action: "Particle"
}

/**
 * Copies a particle and stringify it.
 */
export function particleToString(call: Particle): string {
    let obj: any = {...call};

    return JSON.stringify(obj)
}


export function parseParticle(str: string): Particle {
    let json = JSON.parse(str);

    return {
        id: json.id,
        init_peer_id: json.init_peer_id,
        timestamp: json.timestamp,
        ttl: json.ttl,
        script: json.script,
        signature: json.signature,
        data: json.data,
        action: "Particle"
    }
}

/**
 * Sign a particle with a private key from peerId.
 */
export async function signParticle(peerId: PeerId,
                                   id: string,
                                   timestamp: bigint,
                                   ttl: number,
                                   script: string): Promise<string> {
    let peerIdBuf = Buffer.from(peerId.toB58String(), 'utf8');
    let idBuf = Buffer.from(id, 'utf8');

    let tsArr = new ArrayBuffer(8);
    new DataView(tsArr).setBigUint64(0, timestamp);
    let tsBuf = new Buffer(tsArr);

    let ttlArr = new ArrayBuffer(4);
    new DataView(ttlArr).setUint32(0, ttl);
    let ttlBuf = new Buffer(ttlArr);

    let scriptBuf = Buffer.from(script, 'utf8');

    let bufToSign = Buffer.concat([peerIdBuf, idBuf, tsBuf, ttlBuf, scriptBuf]);

    let signature = await peerId.privKey.sign(bufToSign)
    return encode(signature)
}

export function genUUID() {
    return uuidv4();
}
