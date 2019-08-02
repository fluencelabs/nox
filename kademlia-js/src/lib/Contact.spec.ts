import { Contact } from './Contact';

const contactPublicKey = 'ARQqFPuW7asoZtw5JGB32fXvgJzHK9E3HEYoBHh1dKAV';
const contactHost = '207.154.240.52';
const contactPort = 25000;
const contactSignature = '3KVKDyDu99uwZ7UocE1DH5sDY3eYJXfrqHLrkkDDtYCBbvTJ8vmvjqVMFZgRohcVmpzsyrQ3nqp2ARW3YEUby3rY';
const contactUri = `fluence://${contactPublicKey}:${contactSignature}@${contactHost}:${contactPort}`;
const contactUriWrong = `fluence://${contactPublicKey}:${contactSignature}@${contactHost}:8081`;
const kademliaKey = '3U5Fu884bU8wb889nq7ZfsWL3QZv';

describe('Contact class tests', () => {
    it('creates Contact instance', function () {
        const contact = new Contact(
            contactPublicKey,
            contactHost,
            contactPort,
            contactSignature
        );

        expect(contact).toBeInstanceOf(Contact);
        expect(contact.getHostPort()).toStrictEqual(`${contactHost}:${contactPort}`);
        expect(contact.getPublicKey()).toStrictEqual(contactPublicKey);
    });

    it('creates Contact instance from uri', function () {
        const contact = Contact.fromUri(contactUri);

        expect(contact).toBeInstanceOf(Contact);
        expect(contact.getHostPort()).toStrictEqual(`${contactHost}:${contactPort}`);
        expect(contact.getPublicKey()).toStrictEqual(contactPublicKey);
    });

    it('verifies Contact signature', function () {
        const contact = Contact.fromUri(contactUri);

        expect(contact.isSignatureValid()).toStrictEqual(true);
    });

    it('fails on wrong Contact signature', function () {
        const contact = Contact.fromUri(contactUriWrong);

        expect(contact.isSignatureValid()).toStrictEqual(false);
    });

    it('returns valid kademlia key', function () {
        const contact = Contact.fromUri(contactUriWrong);

        expect(contact.getKademliaKey()).toStrictEqual(kademliaKey);
    });
});
