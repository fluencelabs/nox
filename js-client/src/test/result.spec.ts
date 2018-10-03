import {Empty, parseObject, Value, ErrorResult} from "../Result";
import { expect } from 'chai';

import 'mocha';


describe('Result', () => {
    let randomstring = require("randomstring");

    describe('#parseObject()', () => {
        it('should return Value if `Computed.value` field is exists', () => {
            let value = randomstring.generate(20);
            let obj = {
                Computed: {
                    value: value
                }
            };

            let res = parseObject(obj);
            expect(res).to.be.an.instanceOf(Value).that.include({value: value})
        });
        it('should return Empty if `Empty` field is exists', () => {
            let obj = {
                Empty: {}
            };

            let res = parseObject(obj);
            expect(res).to.be.an.instanceOf(Empty)
        });
        it('should throw error if `Error` field is exists', () => {
            let value = randomstring.generate(20);
            let obj = {
                Error: {
                    message: value
                }
            };

            expect(() => parseObject(obj)).to.throw(ErrorResult).that.include({error: value})
        });
        it('should throw error if cannot parse object', () => {
            let obj = {
                randomobject: {
                    randomfield: "randomvalue"
                }
            };

            expect(() => parseObject(obj)).to.throw()
        });
    });
});