import axios from 'axios';
import { KademliaRpc } from './KademliaRpc';
import { Contact } from './Contact';
import { KademliaRpcHttpTransport } from './KademliaRpcHttpTransport';

const contactPublicKey = 'ARQqFPuW7asoZtw5JGB32fXvgJzHK9E3HEYoBHh1dKAV';
const contactHost = '207.154.240.52';
const contactPort = 25000;
const contactSignature = '3KVKDyDu99uwZ7UocE1DH5sDY3eYJXfrqHLrkkDDtYCBbvTJ8vmvjqVMFZgRohcVmpzsyrQ3nqp2ARW3YEUby3rY';
const contactUri = `fluence://${contactPublicKey}:${contactSignature}@${contactHost}:${contactPort}`;
const contact = Contact.fromUri(contactUri);

const axiosMock: any = axios;

describe('KademliaRPC class tests', () => {
    beforeEach(() => {
        axiosMock.get.mockRestore();
    });

    it('creates KademliaRPC instance from contract', function () {
        const rpc = new KademliaRpc(contact, KademliaRpcHttpTransport);

        expect(rpc).toBeInstanceOf(KademliaRpc);
    });

    it('creates KademliaRPC instance from string', function () {
        const rpc = new KademliaRpc(`${contactHost}:${contactPort}`, KademliaRpcHttpTransport);

        expect(rpc).toBeInstanceOf(KademliaRpc);
    });

    it('calls ping() method', function () {
        const data = `"${contactUri}"`;
        axiosMock.get.mockImplementation(() => Promise.resolve({ data }));
        const rpc = new KademliaRpc(contact, KademliaRpcHttpTransport);

        return rpc.ping().then(res => {
            expect(axiosMock.get).toHaveBeenCalledWith(`http://${contactHost}:${contactPort}/kad/ping`);
            expect(res).toStrictEqual(data);
        });
    });

    it('calls lookup() method', function () {
        const data = `"${contactUri}"`;
        axiosMock.get.mockImplementation(() => Promise.resolve({ data: [ data ] }));
        const rpc = new KademliaRpc(contact, KademliaRpcHttpTransport);
        const kademliaKey = contact.getKademliaKey();
        const neighbors = 1;

        return rpc.lookup(kademliaKey, neighbors).then(res => {
            expect(axiosMock.get).toHaveBeenCalledWith(
                `http://${contactHost}:${contactPort}/kad/lookup`,
                {
                    params: {
                        key: kademliaKey,
                        n: neighbors
                    }
                }
            );
            expect(res).toStrictEqual([ data ]);
        });
    });
});
