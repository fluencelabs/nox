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

import {FluenceClient} from "./fluence_client";
import * as PeerId from "peer-id";
import {keys} from "libp2p-crypto"
import {encode, decode} from "bs58"
import {genUUID} from "./function_call";


const FORMAT = "11";
const VERSION = "1111";

interface Trust {
    issuedFor: PeerId,
    expiresAt: number,
    signature: string,
    issuedAt: number
}

function trustToString(trust: Trust): string {
    return `${encode(trust.issuedFor.pubKey.marshal())}\n${trust.signature}\n${trust.expiresAt}\n${trust.issuedAt}`
}

function certificateToString(cert: Certificate): string {
    let certStr = cert.chain.map(t => trustToString(t)).join("\n");
    return `${FORMAT}\n${VERSION}\n${certStr}`
}

async function trustFromString(str: string): Promise<Trust> {
    let lines = str.split("\n");
    let pubKey = keys.unmarshalPublicKey(decode(lines[0]));
    let peerId = await PeerId.createFromPubKey(pubKey.marshal());

    return {
        issuedFor: peerId,
        signature: lines[1],
        expiresAt: parseInt(lines[2]),
        issuedAt: parseInt(lines[3])
    }
}

interface Certificate {
    chain: Trust[]
}

async function issueRoot(issuedBy: PeerId,
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

async function createTrust(forPk: PeerId, issuedBy: PeerId, expiresAt: number, issuedAt: number): Promise<Trust> {
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


async function issue(issuedBy: PeerId,
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


export class CertGiver {

    client: FluenceClient;

    constructor(client: FluenceClient) {
        this.client = client;
    }

    async addRootCert() {

        let seed = [46, 188, 245, 171, 145, 73, 40, 24, 52, 233, 215, 163, 54, 26, 31, 221, 159, 179, 126, 106, 27, 199, 189, 194, 80, 133, 235, 42, 42, 247, 80, 201];

        // keys.unmarshalPublicKey()

        let privateK = await keys.generateKeyPairFromSeed("Ed25519", Uint8Array.from(seed), 256);
        let peerId = await PeerId.createFromPrivKey(privateK.bytes);

        let clientKey = this.client.selfPeerInfo.id;

        let issuedAt = new Date();
        let expiresAt = new Date();
        expiresAt.setDate(new Date().getDate() + 1);

        let cert = await issueRoot(peerId, clientKey, expiresAt.getTime(), issuedAt.getTime());

        let certStr = certificateToString(cert);

        console.log(certStr)

        await this.client.sendServiceCall("add_certificates", {
            certificates: [certStr],
            msg_id: genUUID(),
            peer_id: clientKey.toB58String()
        });
    }

    /*async getCert(peerId: string): Certificate {
        let msgId = genUUID();
        let resp = await this.client.sendServiceCallWaitResponse("certificates", {
            msg_id: msgId,
            peer_id: peerId
        }, (args) => args.msg_id && args.msg_id === msgId)
    }*/


}