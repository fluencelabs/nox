// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { isChecksumValid, isAddress, toChecksumAddress } from './address';

describe('util/address', () => {
  const value = '63Cf90D3f0410092FC0fca41846f596223979195';
  const address = `0x${value}`;
  const lowercase = `0x${value.toLowerCase()}`;
  const uppercase = `0x${value.toUpperCase()}`;
  const invalid =
    '0x' +
    value
      .split('')
      .map(char => {
        if (char >= 'a' && char <= 'f') {
          return char.toUpperCase();
        } else if (char >= 'A' && char <= 'F') {
          return char.toLowerCase();
        }

        return char;
      })
      .join('');
  const invalidhex = '0x01234567890123456789012345678901234567gh';

  /**
   * @test {isChecksumValid}
   */
  describe('isChecksumValid', () => {
    it('returns false when fully lowercase', () => {
      expect(isChecksumValid(lowercase)).toBe(false);
    });

    it('returns false when fully uppercase', () => {
      expect(isChecksumValid(uppercase)).toBe(false);
    });

    it('returns false on a mixed-case address', () => {
      expect(isChecksumValid(invalid)).toBe(false);
    });

    it('returns true on a checksummed address', () => {
      expect(isChecksumValid(address)).toBe(true);
    });
  });

  /**
   * @test {isAddress}
   */
  describe('isAddress', () => {
    it('returns true when fully lowercase', () => {
      expect(isAddress(lowercase)).toBe(true);
    });

    it('returns true when fully uppercase', () => {
      expect(isAddress(uppercase)).toBe(true);
    });

    it('returns true when checksummed', () => {
      expect(isAddress(address)).toBe(true);
    });

    it('returns false when invalid checksum', () => {
      expect(isAddress(invalid)).toBe(false);
    });

    it('returns false on valid length, non-hex', () => {
      expect(isAddress(invalidhex)).toBe(false);
    });
  });

  /**
   * @test {toChecksumAddress}
   */
  describe('toChecksumAddress', () => {
    it('returns empty when no address specified', () => {
      expect(toChecksumAddress(undefined)).toEqual('');
    });

    it('returns empty when null address specified', () => {
      expect(toChecksumAddress(null)).toEqual('');
    });

    it('returns empty on invalid address structure', () => {
      expect(toChecksumAddress('0xnotaddress')).toEqual('');
    });

    it('returns formatted address on checksum input', () => {
      expect(toChecksumAddress(address)).toEqual(address);
    });

    it('returns formatted address on lowercase input', () => {
      expect(toChecksumAddress(lowercase)).toEqual(address);
    });

    it('returns formatted address on uppercase input', () => {
      expect(toChecksumAddress(uppercase)).toEqual(address);
    });

    it('returns formatted address on mixed input', () => {
      expect(toChecksumAddress(invalid)).toEqual(address);
    });
  });
});
