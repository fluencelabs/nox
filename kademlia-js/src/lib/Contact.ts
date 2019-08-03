import parse from 'url-parse';
import bs58 from 'bs58';
import sha1 from 'js-sha1';
import supercop from 'supercop.js';

export class Contact {
    private host: string;
    private port: number;
    private pubKey: string;
    private signature: string;

    static fromUri(uri: string): Contact {
        const { username, password, hostname, port } = parse(uri);

        return new Contact(username, hostname, Number(port), password);
    }

    constructor(pubKey: string, host: string, port: number, signature: string) {
        this.host = host;
        this.port = port;
        this.pubKey = pubKey;
        this.signature = signature;
    }

    getKademliaKey(): string {
        return bs58.encode(Buffer.from(sha1.arrayBuffer(this.pubKey)));
    }

    getHostPort(): string {
        return `${this.host}:${this.port}`;
    }

    getPublicKey(): string {
        return this.pubKey;
    }

    getSignature(): string {
        return this.signature;
    }

    asUri(): string {
        return `fluence://${this.pubKey}:${this.signature}@${this.host}:${this.port}`;
    }

    isSignatureValid(): boolean {
        const binaryPubKey = bs58.decode(this.pubKey);
        const binarySignature = bs58.decode(this.signature);
        const binaryHost = Buffer.from(this.host);
        const binaryPort = Buffer.from(`00000000${Number(this.port).toString(16)}`.slice(-8), 'hex');

        return supercop.verify(
            binarySignature,
            Buffer.concat([
                binaryPubKey,
                binaryHost,
                binaryPort
            ]),
            binaryPubKey
        );
    }
}
