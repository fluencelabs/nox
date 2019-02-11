// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import BigNumber from 'bignumber.js';

import {
  padAddress,
  padBool,
  padBytes,
  padFixedBytes,
  padString,
  padU32
} from './pad';

describe('util/pad', () => {
  const SHORT15 = '1234567890abcdef';
  const BYTES15 = [0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef];
  const LONG15 = `${SHORT15}000000000000000000000000000000000000000000000000`;
  const PAD123 =
    '0000000000000000000000000000000000000000000000000000000000000123';

  describe('padAddress', () => {
    it('pads to 64 characters', () => {
      expect(padAddress('123')).toEqual(PAD123);
    });

    it('strips leading 0x when passed in', () => {
      expect(padAddress(`0x${PAD123}`)).toEqual(PAD123);
    });
  });

  describe('padBool', () => {
    const TRUE =
      '0000000000000000000000000000000000000000000000000000000000000001';
    const FALSE =
      '0000000000000000000000000000000000000000000000000000000000000000';

    it('pads true to 64 characters', () => {
      expect(padBool(true)).toEqual(TRUE);
    });

    it('pads false to 64 characters', () => {
      expect(padBool(false)).toEqual(FALSE);
    });
  });

  describe('padU32', () => {
    it('left pads length < 64 bytes to 64 bytes', () => {
      expect(padU32(1)).toEqual(
        '0000000000000000000000000000000000000000000000000000000000000001'
      );
    });

    it('pads hex representation', () => {
      expect(padU32(0x123)).toEqual(PAD123);
    });

    it('pads decimal representation', () => {
      expect(padU32(291)).toEqual(PAD123);
    });

    it('pads string representation', () => {
      expect(padU32('0x123')).toEqual(PAD123);
    });

    it('pads BigNumber representation', () => {
      expect(padU32(new BigNumber(0x123))).toEqual(PAD123);
    });

    it('converts negative numbers to 2s complement', () => {
      expect(padU32(-123)).toEqual(
        'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff85'
      );
    });
  });

  describe('padFixedBytes', () => {
    it('right pads length < 64 bytes to 64 bytes (string)', () => {
      expect(padFixedBytes(`0x${SHORT15}`)).toEqual(LONG15);
    });

    it('right pads length < 64 bytes to 64 bytes (array)', () => {
      expect(padFixedBytes(BYTES15)).toEqual(LONG15);
    });

    it('right pads length > 64 bytes (64 byte multiples)', () => {
      expect(padFixedBytes(`0x${LONG15}${SHORT15}`)).toEqual(
        `${LONG15}${LONG15}`
      );
    });

    it('strips leading 0x when passed in', () => {
      expect(padFixedBytes(`0x${SHORT15}`)).toEqual(LONG15);
    });

    it('encodes empty value when 0x is paased', () => {
      expect(padFixedBytes('0x')).toEqual('');
    });
  });

  describe('padBytes', () => {
    it('right pads length < 64, adding the length (string)', () => {
      const result = padBytes(`0x${SHORT15}`);

      expect(result.length).toEqual(128);
      expect(result).toEqual(`${padU32(8)}${LONG15}`);
    });

    it('right pads length < 64, adding the length (array)', () => {
      const result = padBytes(BYTES15);

      expect(result.length).toEqual(128);
      expect(result).toEqual(`${padU32(8)}${LONG15}`);
    });

    it('right pads length > 64, adding the length', () => {
      const result = padBytes(`0x${LONG15}${SHORT15}`);

      expect(result.length).toEqual(192);
      expect(result).toEqual(`${padU32(0x28)}${LONG15}${LONG15}`);
    });
  });

  describe('padString', () => {
    it('correctly converts & pads strings', () => {
      const result = padString('gavofyork');

      expect(result.length).toEqual(128);
      expect(result).toEqual(padBytes('0x6761766f66796f726b'));
    });
  });
});
