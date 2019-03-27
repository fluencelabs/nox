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
