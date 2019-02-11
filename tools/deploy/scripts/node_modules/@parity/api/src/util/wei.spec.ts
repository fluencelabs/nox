// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { EtherDenomination } from '../types';
import { _getUnitMultiplier, fromWei, toWei } from './wei';

describe('util/wei', () => {
  /**
   * @test {_getUnitMultiplier}
   */
  describe('_getUnitMultiplier', () => {
    it('returns 10^0 for wei', () => {
      expect(_getUnitMultiplier('wei')).toEqual(Math.pow(10, 0));
    });

    it('returns 10^15 for finney', () => {
      expect(_getUnitMultiplier('finney')).toEqual(Math.pow(10, 15));
    });

    it('returns 10^18 for ether', () => {
      expect(_getUnitMultiplier('ether')).toEqual(Math.pow(10, 18));
    });

    it('throws an error on invalid units', () => {
      expect(() => _getUnitMultiplier('invalid' as EtherDenomination)).toThrow(
        /passed to wei formatter/
      );
    });
  });

  /**
   * @test {fromWei}
   */
  describe('fromWei', () => {
    it('formats into ether when nothing specified', () => {
      expect(fromWei('1230000000000000000').toString()).toEqual('1.23');
    });

    it('formats into finney when specified', () => {
      expect(fromWei('1230000000000000000', 'finney').toString()).toEqual(
        '1230'
      );
    });
  });

  /**
   * @test {toWei}
   */
  describe('toWei', () => {
    it('formats from ether when nothing specified', () => {
      expect(toWei(1.23).toString()).toEqual('1230000000000000000');
    });

    it('formats from finney when specified', () => {
      expect(toWei(1230, 'finney').toString()).toEqual('1230000000000000000');
    });
  });
});
