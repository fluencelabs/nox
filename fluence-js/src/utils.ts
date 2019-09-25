import * as randomstring from "randomstring";
import {TxRequest} from "./TendermintClient";

export type PrivateKey = Buffer

const sha256 = require("sha256");
export const secp256k1 = require("secp256k1");

export function fromHexToView(str: string): DataView {
    let buf = Buffer.from(str, 'hex');

    return new DataView(buf.buffer, 0);
}

export function fromHex(str: string): string {
    return Buffer.from(str, 'hex').toString();
}

export function toHex(str: string): string {
    return Buffer.from(str).toString('hex').toUpperCase();
}

function sign(payload: string, privateKey: PrivateKey): string {
    //TODO: verify private key
    let bytes = Buffer.from(payload);
    let hash = Buffer.from(sha256(bytes, { asBytes: true }));
    let sig = secp256k1.sign(hash, privateKey);
    return sig.signature.toString('hex');
}

export function remove0x(hex: string): string {
    if (hex.startsWith("0x")) {
        return hex.slice(2);
    } else {
        return hex;
    }
}

export function parseHost(host: string): { protocol: string, hostname: string } {
    const parsedHost = /^(((http[s]?):\/\/)|(\/\/))([^:\/]+)/g.exec(host);
    let protocol = 'http';

    if (typeof window !== 'undefined') {
        const pageProtocol = (window as any).location.protocol.slice(0, -1);
        if (pageProtocol === 'http' || pageProtocol === 'https') {
            protocol = pageProtocol;
        }
    }

    if (parsedHost) {
        return {
            protocol: parsedHost[3] || protocol,
            hostname: parsedHost[5],
        }
    }

    return {
        protocol: protocol,
        hostname: host,
    }
}

/**
 * Signs the payload concatenated with counter, and prepends the signature to the signed data
 *
 * @param payload Payload to sign
 * @param nonce Nonce to concatenate with payload before signing
 * @param privateKey Private key to sign with
 * @returns signature \n nonce \n payload
 */
export function withSignature(payload: string, nonce: number, privateKey?: PrivateKey): string {
    if (privateKey == undefined) {
        return payload;
    }

    let withNonce = `${nonce}\n${payload}`;
    let sig = sign(withNonce, privateKey);
    return `${sig}\n${withNonce}`;
}

export function genSessionId(): string {
    return randomstring.generate(12);
}

export function genRequestId(): string {
    return randomstring.generate(8);
}

/**
 * Generates a key, that will be an identifier of the request.
 */
export function generateHeader(session: string, counter: number) {
    return `${session}/${counter}`;
}

/**
 * Checks if everything ok with the session before a request will be sent.
 * Builds a request.
 */
export function prepareRequest(payload: string, session: string, currentCounter: number, privateKey?: PrivateKey): TxRequest {
    // increments counter at the start, if some error occurred, other requests will be canceled in `cancelAllPromises`
    let signed = withSignature(payload, currentCounter, privateKey);
    let header = generateHeader(session, currentCounter);
    let tx = `${header}\n${signed}`;

    return  {
        path: header,
        payload: tx
    }
}
