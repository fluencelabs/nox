// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import {
  bytesToHex,
  cleanupValue,
  hexToBytes,
  hexToAscii,
  bytesToAscii,
  asciiToHex,
  padLeft,
  padRight
} from './format';

describe('util/format', () => {
  /**
   * @test {bytesToHex}
   */
  describe('bytesToHex', () => {
    it('correctly converts an empty array', () => {
      expect(bytesToHex([])).toEqual('0x');
    });

    it('correctly converts a non-empty array', () => {
      expect(bytesToHex([0, 15, 16])).toEqual('0x000f10');
    });
  });

  /**
   * @test {cleanupValue}
   */
  describe('cleanupValue', () => {
    it('returns unknown values as the original', () => {
      expect(cleanupValue('original', 'unknown')).toEqual('original');
    });

    it('returns ascii arrays as ascii', () => {
      expect(cleanupValue([97, 115, 99, 105, 105, 0], 'bytes32')).toEqual(
        'ascii'
      );
    });

    it('returns non-ascii arrays as hex strings', () => {
      expect(cleanupValue([97, 200, 0, 0], 'bytes4')).toEqual('0x61c80000');
    });

    it('returns uint (>48) as the original', () => {
      expect(cleanupValue('original', 'uint49')).toEqual('original');
    });

    it('returns uint (<=48) as the number value', () => {
      expect(cleanupValue('12345', 'uint48')).toEqual(12345);
    });
  });

  /**
   * @test {hexToBytes}
   */
  describe('hexToBytes', () => {
    it('correctly converts an empty string', () => {
      expect(hexToBytes('')).toEqual([]);
      expect(hexToBytes('0x')).toEqual([]);
    });

    it('correctly converts a non-empty string', () => {
      expect(hexToBytes('0x000f10')).toEqual([0, 15, 16]);
    });
  });

  /**
   * @test {asciiToHex}
   */
  describe('asciiToHex', () => {
    it('correctly converts an empty string', () => {
      expect(asciiToHex('')).toEqual('0x');
    });

    it('correctly converts a non-empty string', () => {
      expect(asciiToHex('abc')).toEqual('0x616263');
      expect(asciiToHex('a\nb')).toEqual('0x610a62');
    });

    it('correctly converts where charCode < 0x10', () => {
      expect(
        asciiToHex(
          [32, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]
            .map(v => String.fromCharCode(v))
            .join('')
        )
      ).toEqual('0x20100f0e0d0c0b0a09080706050403020100');
    });
  });

  /**
   * @test {hexToAscii}
   */
  describe('hexToAscii', () => {
    it('correctly converts an empty string', () => {
      expect(hexToAscii('')).toEqual('');
      expect(hexToAscii('0x')).toEqual('');
    });

    it('correctly converts a non-empty string', () => {
      expect(hexToAscii('0x616263')).toEqual('abc');
    });
  });

  /**
   * @test {bytesToAscii}
   */
  describe('bytesToAscii', () => {
    it('correctly converts an empty string', () => {
      expect(bytesToAscii([])).toEqual('');
    });

    it('correctly converts a non-empty string', () => {
      expect(bytesToAscii([97, 98, 99])).toEqual('abc');
    });
  });

  /**
   * @test {padLeft}
   */
  describe('padLeft', () => {
    it('correctly pads to the number of hex digits', () => {
      expect(padLeft('ab', 4)).toEqual('0x000000ab');
    });
  });

  /**
   * @test {padRight}
   */
  describe('padRight', () => {
    it('correctly pads to the number of hex digits', () => {
      expect(padRight('ab', 4)).toEqual('0xab000000');
    });
  });
});
