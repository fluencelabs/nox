// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { asAddress, asBool, asI32, asU32 } from './sliceAs';

describe('util/sliceAs', () => {
  const MAX_INT =
    'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff';

  describe('asAddress', () => {
    it('correctly returns the last 0x40 characters', () => {
      const address = '1111111111222222222233333333334444444444';

      expect(asAddress(`000000000000000000000000${address}`)).toEqual(
        `0x${address}`
      );
    });
  });

  describe('asBool', () => {
    it('correctly returns true', () => {
      expect(
        asBool(
          '0000000000000000000000000000000000000000000000000000000000000001'
        )
      ).toBe(true);
    });

    it('correctly returns false', () => {
      expect(
        asBool(
          '0000000000000000000000000000000000000000000000000000000000000000'
        )
      ).toBe(false);
    });
  });

  describe('asI32', () => {
    it('correctly decodes positive numbers', () => {
      expect(
        asI32(
          '000000000000000000000000000000000000000000000000000000000000007b'
        ).toString()
      ).toEqual('123');
    });

    it('correctly decodes negative numbers', () => {
      expect(
        asI32(
          'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff85'
        ).toString()
      ).toEqual('-123');
    });
  });

  describe('asU32', () => {
    it('returns a maxium U32', () => {
      expect(asU32(MAX_INT).toString(16)).toEqual(MAX_INT);
    });
  });
});
