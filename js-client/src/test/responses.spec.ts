
import { expect } from 'chai';

import 'mocha';
import {isActive, SessionSummary} from "../responses";


describe('SessionSummary', () => {
    describe('#isActive()', () => {
        it('should return `true` if `Active` is presence', () => {
            let obj = {
                status: {
                    Active: {}
                },
                invokedTxsCount: 1,
                lastTxCounter: 1
            };

            expect(isActive(<SessionSummary> obj)).to.be.equal(true)

        });
    });
});
