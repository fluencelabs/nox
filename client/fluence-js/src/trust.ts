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
import crypto from 'libp2p-crypto';
import {genUUID} from "./function_call";
import * as nacl from "tweetnacl";
import {sha512} from "js-sha512";
const ed25519 = crypto.keys.supportedKeys.ed25519;


const FORMAT = "11";
const VERSION = "1111";

interface Trust {
    issuedFor: string,
    expiresAt: number,
    signature: string,
    issuedAt: number
}

function trustToString(trust: Trust): string {
    return `${trust.issuedFor}\n${trust.signature}\n${trust.expiresAt}\n${trust.issuedAt}`
}

function certificateToString(cert: Certificate): string {
    let certStr = cert.chain.map(t => trustToString(t)).join("\n");
    return `${FORMAT}\n${VERSION}\n${certStr}`
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

    /*
    let root_trust = Trust::create(root_kp, root_kp.public_key(), root_expiration, issued_at);

        let trust = Trust::create(root_kp, for_pk, expires_at, issued_at);
     */

    let rootTrust = await createTrust(issuedBy, issuedBy, maxDate, issuedAt);
    let trust = await createTrust(forPk, issuedBy , expiresAt, issuedAt);

    let chain = [rootTrust, trust];

    return {
        chain: chain
    }
}

async function createTrust(forPk: PeerId, issuedBy: PeerId, expiresAt: number, issuedAt: number): Promise<Trust> {
    let bytes = toSignMessage(forPk, expiresAt, issuedAt);


    // console.log("full buf: " + JSON.stringify(bytes.join(", ")));

    // console.log("priv key: " + JSON.stringify(issuedBy.privKey.marshal().slice(0, 64).join(", ")));

    // let hash = await issuedBy.privKey.hash();
    // let a = hash.slice(2, 34);
    // console.log("len === " + hash.length);
    // console.dir(hash);
    // console.dir(a);
    // console.dir(issuedBy.pubKey.marshal());
    // let kp = nacl.sign.keyPair.fromSeed(a);

    // let kp = nacl.sign.keyPair.fromSeed(rust_seed);
    // console.log("kp priv key: " + JSON.stringify(kp.secretKey.join(", ")));
    // console.log("kp pub key: " + JSON.stringify(kp.publicKey.join(", ")));
    // let rust_seed = Uint8Array.from([59, 84, 253, 246, 102, 114, 28, 171, 174, 33, 213, 22, 180, 26, 139, 235, 29, 18, 92, 15, 6, 115, 249, 223, 96, 251, 52, 79, 150, 78, 126, 59]);

    // let bytes = Uint8Array.from([1,2,3,4]);
    // let signature = nacl.sign.detached(bytes, kp.secretKey);
    let signature = await issuedBy.privKey.sign(Buffer.from(bytes));
    // console.log("full buf: " + JSON.stringify(bytes.join(", ")));
    // console.log("check: " + nacl.sign.detached.verify(bytes, signature, kp.publicKey));
    console.log("check: " + await issuedBy.pubKey.verify(Buffer.from(bytes), Buffer.from(signature)));
    // console.log("full buf: " + JSON.stringify(bytes.join(", ")));

    // let signature = encode(await issuedBy.privKey.sign(Buffer.from(buf)));

    let forPkStr = encode(forPk.pubKey.marshal());

    let signatureStr = encode(signature);

    return {
        issuedFor: forPkStr,
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


    if (lastTrust.issuedFor !== encode(issuedBy.pubKey.bytes)) {
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
    console.log("pkEncoded = " + JSON.stringify(pkEncoded));
    bytes.set(pkEncoded, 0);
    bytes.set(numToArray(expiresAt), 32);
    bytes.set(numToArray(issuedAt), 40);

    console.log("pk = " + pk.toB58String());

    console.log("expiresAt = " + JSON.stringify(numToArray(expiresAt)));
    console.log("issuedAt = " + JSON.stringify(numToArray(issuedAt)));

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

    /**
     * public: Ct8ewXqEzSUvLR9CVtW39tHEDu3iBRsj21DzBZMc8LB4
     keypair wCUPkGaBypwbeuUnmgVyN37j9iavRoqzAkddDUzx3Yir7q1yuTp3H8cdUZERYxeQ8PEiMYcDuiy1DDkfueNh1Y6
     */

    constructor(client: FluenceClient) {
        this.client = client;
    }

    async addRootCert() {

        /*const keyMarshal = decode("dSj4ZrzcBSHu1Cq2MFGh7nEP5Drx2ShYCcuNAH622SwfoWNxtUiZ6HFbtjRTiHXHnop4Dgbc6Gemm9nMf47tQH6RGX3NU1EN37sW1ZWqif9F3gisw8Q26GaqyfUChvBe7MM");
        let peerid2 = await PeerId.createFromPrivKey(keyMarshal);
        console.log("molodchina! " + peerid2.toB58String());*/

        let privKeyBuf = decode("dSj4ZrzcBSHu1Cq2MFGh7nEP5Drx2ShYCcuNAH622SwfoWNxtUiZ6HFbtjRTiHXHnop4Dgbc6Gemm9nMf47tQH6RGX3NU1EN37sW1ZWqif9F3gisw8Q26GaqyfUChvBe7MM");


        let privKey: crypto.keys.supportedKeys.ed25519.Ed25519PrivateKey = await ed25519.unmarshalEd25519PrivateKey(privKeyBuf);

        let seed = [46, 188, 245, 171, 145, 73, 40, 24, 52, 233, 215, 163, 54, 26, 31, 221, 159, 179, 126, 106, 27, 199, 189, 194, 80, 133, 235, 42, 42, 247, 80, 201];
        let hash = [138, 180, 146, 97, 1, 59, 93, 87, 210, 91, 3, 231, 152, 194, 82, 32, 187, 238, 145, 40, 101, 154, 201, 12, 89, 213, 60, 43, 61, 41, 101, 139, 219, 66, 88, 77, 181, 3, 66, 219, 108, 234, 126, 136, 63, 161, 130, 228, 231, 75, 172, 12, 168, 196, 13, 30, 150, 71, 180, 206, 172, 228, 245, 25]
        let hash_seed = sha512.arrayBuffer(seed);
        let hash_seed_sliced = new Uint8Array(hash_seed.slice(0, 32));
        console.log("sliced: ");
        console.log(hash_seed_sliced);
        let privateK = await keys.generateKeyPairFromSeed("Ed25519", Uint8Array.from(seed), 256);
        let peerId = await PeerId.createFromPrivKey(privateK.bytes);
        // console.dir(pp);


        // let peerId = await PeerId.create({keyType: "Ed25519"});
        console.log("PEER ID = " + peerId.toB58String());

        let clientKey = this.client.selfPeerInfo.id;


        let issuedAt = new Date();
        let expiresAt = new Date();
        expiresAt.setDate(new Date().getDate()+1);

        let cert = await issueRoot(peerId, clientKey, expiresAt.getTime(), issuedAt.getTime());

        let singleCert: Certificate = {
            chain: [cert.chain[0]]
        };

        let certStr = certificateToString(cert);

        console.log(certStr)

        await this.client.sendServiceCall("add_certificates", { certificates: [certStr], msg_id: genUUID(), peer_id: clientKey.toB58String() });
    }


}