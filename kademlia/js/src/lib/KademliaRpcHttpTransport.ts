import axios from 'axios';
import { KademliaRpcTransport } from './KademliaRpc';
import { Contact } from './Contact';

export class KademliaRpcHttpTransport implements KademliaRpcTransport {
    private contact: Contact;
    private hostPort: string;

    constructor(contact: Contact | string) {
        if (typeof(contact) !== 'string') {
            this.contact = contact;
            this.hostPort = this.contact.getHostPort();
        } else {
            this.hostPort = contact;
        }
    }

    async ping(): Promise<string> {
        return axios.get(`http://${this.hostPort}/kad/ping`).then(response => response.data);
    }

    async lookup(key: string, neighbors?: number): Promise<string[]> {
        return axios.get(
            `http://${this.hostPort}/kad/lookup`,
            {
                params: {
                    key,
                    n: neighbors
                }
            }
        ).then(response => response.data);
    }
}
