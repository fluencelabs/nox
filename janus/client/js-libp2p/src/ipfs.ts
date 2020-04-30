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

const ipfsClient = require('ipfs-http-client');
const Hash = require('ipfs-only-hash');

export async function calcHash(data: Buffer): Promise<string> {
    return await Hash.of(data);
}

// Add a file to IPFS node with $multiaddr address
export async function ipfsAdd(multiaddr: string, file: Uint8Array): Promise<void> {
    const ipfs = ipfsClient(multiaddr);
    const source = ipfs.add(
        [file]
    );

    try {
        for await (const file of source) {
            console.log(`file uploaded to '${multiaddr}'`);
        }
    } catch (err) {
        console.error(err)
    }

    return Promise.resolve();
}

// Get a file from a node with $multiaddr address
export async function ipfsGet(multiaddr: string, path: string): Promise<Uint8Array> {
    const ipfs = ipfsClient(multiaddr);
    const source = ipfs.cat(path);

    let bytes = new Uint8Array();

    try {
        for await (const chunk of source) {
            const newArray = new Uint8Array(bytes.length + chunk.length);
            newArray.set(bytes, 0);
            newArray.set(chunk, bytes.length);

            bytes = newArray;
        }
    } catch (err) {
        console.error(err)
    }

    return Promise.resolve(bytes);
}
