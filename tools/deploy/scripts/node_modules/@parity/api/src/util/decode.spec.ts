// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import BigNumber from 'bignumber.js';
import {
  abiDecode,
  decodeCallData,
  decodeMethodInput,
  methodToAbi
} from './decode';

describe('util/decode', () => {
  const METH = '0x70a08231';
  const ENCO =
    '0x70a082310000000000000000000000005A5eFF38DA95b0D58b6C616f2699168B480953C9';
  const DATA =
    '0x0000000000000000000000005A5eFF38DA95b0D58b6C616f2699168B480953C9';

  describe('decodeCallData', () => {
    it('throws on non-hex inputs', () => {
      expect(() => decodeCallData('invalid')).toThrow(/should be a hex value/);
    });

    it('throws when invalid signature length', () => {
      expect(() => decodeCallData(METH.slice(-6))).toThrow(
        /should be method signature/
      );
    });

    it('splits valid inputs properly', () => {
      expect(decodeCallData(ENCO)).toEqual({
        signature: METH,
        paramdata: DATA
      });
    });
  });

  describe('decodeMethodInput', () => {
    it('expects a valid ABI', () => {
      expect(() => decodeMethodInput(null as any, null as any)).toThrow(
        /should receive valid method/
      );
    });

    it('expects valid hex parameter data', () => {
      expect(() => decodeMethodInput({}, 'invalid')).toThrow(
        /should be a hex value/
      );
    });

    it('correct decodes null inputs', () => {
      expect(decodeMethodInput({})).toEqual([]);
    });

    it('correct decodes empty inputs', () => {
      expect(decodeMethodInput({}, '')).toEqual([]);
    });

    it('correctly decodes valid inputs', () => {
      expect(
        decodeMethodInput(
          {
            type: 'function',
            inputs: [{ type: 'uint' }]
          },
          DATA
        )
      ).toEqual([new BigNumber('0x5a5eff38da95b0d58b6c616f2699168b480953c9')]);
    });
  });

  describe('methodToAbi', () => {
    it('throws when no start ( specified', () => {
      expect(() => methodToAbi('invalid,uint,bool)')).toThrow(
        /Missing start \(/
      );
    });

    it('throws when no end ) specified', () => {
      expect(() => methodToAbi('invalid(uint,bool')).toThrow(/Missing end \)/);
    });

    it('throws when end ) is not in the last position', () => {
      expect(() => methodToAbi('invalid(uint,bool)2')).toThrow(
        /Extra characters after end \)/
      );
    });

    it('throws when start ( is after end )', () => {
      expect(() => methodToAbi('invalid)uint,bool(')).toThrow(
        /End \) is before start \(/
      );
    });

    it('throws when invalid types are present', () => {
      expect(() => methodToAbi('method(invalidType,bool,uint)')).toThrow(
        /Cannot convert invalidType/
      );
    });

    it('returns a valid methodabi for a valid method', () => {
      expect(methodToAbi('valid(uint,bool)')).toEqual({
        type: 'function',
        name: 'valid',
        inputs: [{ type: 'uint256' }, { type: 'bool' }]
      });
    });
  });

  describe('abiDecode', () => {
    it('correctly decodes valid inputs', () => {
      expect(abiDecode(['uint'], DATA)).toEqual([
        new BigNumber('0x5a5eff38da95b0d58b6c616f2699168b480953c9')
      ]);
    });
  });
});
