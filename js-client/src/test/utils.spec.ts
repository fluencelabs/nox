import { fromHex, toHex } from '../utils';
import { expect } from 'chai';

import 'mocha';

describe('utils', () => {

    describe('#toHex()', () => {
        it('should return expected result', () => {
            const str = "some checked string";
            const result = toHex(str);
            expect(result).to.equal('736F6D6520636865636B656420737472696E67');
        });
    });

    describe('#fromHex()', () => {
        it('should return expected result', () => {
            const hex = "616E6F746865722072616E646F6D20737472696E67";
            const result = fromHex(hex);
            expect(result).to.equal('another random string');
        });
    });

    it('the result should be equal to the original input variable after #toHex() and #fromHex()', () => {
        let randomstring = require("randomstring");

        const str =  randomstring.generate(20);

        expect(fromHex(toHex(str))).to.equal(str);
    });
});