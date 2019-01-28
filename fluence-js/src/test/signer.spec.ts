import { expect } from 'chai';

import 'mocha';
import {Signer} from "../Signer";
import * as elliptic from "elliptic";
import {utils} from "elliptic";
import sha256 from "fast-sha256";
import * as base64js from "base64-js";

describe('Signer', () => {
    let randomstring = require("randomstring");
    let ec = new elliptic.eddsa('ed25519');

    describe('#sign()', () => {

        it('should sign string correct', () => {

            let signKey = "TVAD4tNeMH2yJfkDZBSjrMJRbavmdc3/fGU2N2VAnxT3hAtSkX+Lrl4lN5OEsXjD7GGG7iEewSod472HudrkrA==";

            let signer = new Signer(signKey);

            let message = randomstring.generate(20);

            let sign = signer.sign(message);


            // the signer signs a hash of the message, so get hash
            let messageBytes = utils.toArray(message);
            let hashedMessage = sha256(messageBytes);

            // get only public key for verifying
            let key = ec.keyFromSecret(utils.toHex(base64js.toByteArray(signKey)));
            let publicKey = ec.keyFromPublic(key.getPublic());

            // get hex of sign for verifier
            let signHex = Buffer.from(base64js.toByteArray(sign)).toString('hex');
            let res = ec.verify(utils.toHex(hashedMessage), signHex, publicKey);
            expect(res).to.be.equal(true);

        });
    });
});