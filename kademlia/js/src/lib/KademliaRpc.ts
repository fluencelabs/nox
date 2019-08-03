import { Contact } from './Contact';

interface KademliaRpcTransportConstructor {
    new(c: Contact|string): KademliaRpcTransport;
}

export interface KademliaRpcTransport {
    ping: () => Promise<string>;
    lookup: (key: string, neighbors?: number) => Promise<string[]>;
}

export class KademliaRpc {
    private transport: KademliaRpcTransport;

    constructor(contact: Contact | string, transport: KademliaRpcTransportConstructor) {
        this.transport = new transport(contact);
    }

    async ping(): Promise<string> {
        return this.transport.ping();
    }

    async lookup(key: string, neighbors?: number): Promise<string[]> {
        return this.transport.lookup(key, neighbors);
    }
}
