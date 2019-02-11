// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { sliceData } from './slice';

describe('util/slice', () => {
  describe('sliceData', () => {
    const slice1 =
      '131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b';
    const slice2 =
      '2124768576358735263578356373526387638357635873563586353756358763';

    it('returns an empty array when length === 0', () => {
      expect(sliceData('')).toEqual([]);
    });

    it('returns array with zero entry when data === 0x', () => {
      expect(sliceData('0x')).toEqual([
        '0000000000000000000000000000000000000000000000000000000000000000'
      ]);
    });

    it('returns an array with the slices otherwise', () => {
      const sliced = sliceData(`${slice1}${slice2}`);
      if (!sliced) {
        throw new Error('No matches');
      }

      expect(sliced.length).toEqual(2);
      expect(sliced[0]).toEqual(slice1);
      expect(sliced[1]).toEqual(slice2);
    });

    it('removes leading 0x when passed in', () => {
      const sliced = sliceData(`0x${slice1}${slice2}`);
      if (!sliced) {
        throw new Error('No matches');
      }

      expect(sliced.length).toEqual(2);
      expect(sliced[0]).toEqual(slice1);
      expect(sliced[1]).toEqual(slice2);
    });
  });
});
