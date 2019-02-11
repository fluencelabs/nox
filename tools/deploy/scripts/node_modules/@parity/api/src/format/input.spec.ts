// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import BigNumber from 'bignumber.js';
import { isAddress } from '@parity/abi/lib/util/address';

import { FilterOptions, Options } from '../types';
import {
  inAddress,
  inAddresses,
  inBlockNumber,
  inData,
  inFilter,
  inHash,
  inHex,
  inNumber10,
  inNumber16,
  inOptions,
  inTraceType,
  inDeriveHash,
  inDeriveIndex,
  inTopics
} from './input';

describe('format/input', () => {
  const address = '0x63cf90d3f0410092fc0fca41846f596223979195';

  describe('inAddress', () => {
    const address = '63cf90d3f0410092fc0fca41846f596223979195';

    it('adds the leading 0x as required', () => {
      expect(inAddress(address)).toEqual(`0x${address}`);
    });

    it('returns verified addresses as-is', () => {
      expect(inAddress(`0x${address}`)).toEqual(`0x${address}`);
    });

    it('returns lowercase equivalents', () => {
      expect(inAddress(address.toUpperCase())).toEqual(`0x${address}`);
    });

    it('returns 0x on null addresses', () => {
      expect(inAddress()).toEqual('0x');
    });
  });

  describe('inAddresses', () => {
    it('handles empty values', () => {
      expect(inAddresses()).toEqual([]);
    });

    it('handles mapping of all addresses in array', () => {
      const address = '63cf90d3f0410092fc0fca41846f596223979195';

      expect(inAddresses([undefined, address])).toEqual(['0x', `0x${address}`]);
    });
  });

  describe('inBlockNumber()', () => {
    it('returns earliest as-is', () => {
      expect(inBlockNumber('earliest')).toEqual('earliest');
    });

    it('returns latest as-is', () => {
      expect(inBlockNumber('latest')).toEqual('latest');
    });

    it('returns pending as-is', () => {
      expect(inBlockNumber('pending')).toEqual('pending');
    });

    it('formats existing BigNumber into hex', () => {
      expect(inBlockNumber(new BigNumber(0x123456))).toEqual('0x123456');
    });

    it('formats hex strings into hex', () => {
      expect(inBlockNumber('0x123456')).toEqual('0x123456');
    });

    it('formats numbers into hex', () => {
      expect(inBlockNumber(0x123456)).toEqual('0x123456');
    });
  });

  describe('inData', () => {
    it('formats to hex', () => {
      expect(inData('123456')).toEqual('0x123456');
    });

    it('converts a string to a hex representation', () => {
      expect(inData('jaco')).toEqual('0x6a61636f');
    });
  });

  describe('inHex', () => {
    it('leaves leading 0x as-is', () => {
      expect(inHex('0x123456')).toEqual('0x123456');
    });

    it('adds a leading 0x', () => {
      expect(inHex('123456')).toEqual('0x123456');
    });

    it('returns uppercase as lowercase (leading 0x)', () => {
      expect(inHex('0xABCDEF')).toEqual('0xabcdef');
    });

    it('returns uppercase as lowercase (no leading 0x)', () => {
      expect(inHex('ABCDEF')).toEqual('0xabcdef');
    });

    it('handles empty & null', () => {
      expect(inHex()).toEqual('0x');
      expect(inHex('')).toEqual('0x');
    });
  });

  describe('inHash', () => {
    it('leaves leading 0x as-is', () => {
      expect(inHash('0x123456')).toEqual('0x123456');
    });
  });

  describe('inFilter', () => {
    ['address' as keyof FilterOptions].forEach(input => {
      it(`formats ${input} address as address`, () => {
        const block: FilterOptions = {};

        block[input] = address;
        const formatted = inFilter(block)[input];

        expect(isAddress(formatted as string)).toBe(true);
        expect(formatted).toEqual(address);
      });
    });

    (['fromBlock', 'toBlock'] as (keyof FilterOptions)[]).forEach(input => {
      it(`formats ${input} number as blockNumber`, () => {
        const block: FilterOptions = {};

        block[input] = 0x123;
        const formatted = inFilter(block)[input];

        expect(formatted).toEqual('0x123');
      });
    });

    it('ignores and passes through unknown keys', () => {
      expect(inFilter({ someRandom: 'someRandom' } as any)).toEqual({
        someRandom: 'someRandom'
      });
    });

    it('formats an filter options object with relevant entries converted', () => {
      expect(
        inFilter({
          address: address,
          fromBlock: 'latest',
          toBlock: 0x101,
          extraData: 'someExtraStuffInHere',
          limit: 0x32
        })
      ).toEqual({
        address: address,
        fromBlock: 'latest',
        toBlock: '0x101',
        extraData: 'someExtraStuffInHere',
        limit: 50
      });
    });
  });

  describe('inNumber10()', () => {
    it('formats existing BigNumber into number', () => {
      expect(inNumber10(new BigNumber(123))).toEqual(123);
    });

    it('formats hex strings into decimal', () => {
      expect(inNumber10('0x0a')).toEqual(10);
    });

    it('formats numbers into number', () => {
      expect(inNumber10(123)).toEqual(123);
    });

    it('formats undefined into 0', () => {
      expect(inNumber10()).toEqual(0);
    });
  });

  describe('inNumber16()', () => {
    it('formats existing BigNumber into hex', () => {
      expect(inNumber16(new BigNumber(0x123456))).toEqual('0x123456');
    });

    it('formats hex strings into hex', () => {
      expect(inNumber16('0x123456')).toEqual('0x123456');
    });

    it('formats numbers into hex', () => {
      expect(inNumber16(0x123456)).toEqual('0x123456');
    });

    it('formats undefined into 0', () => {
      expect(inNumber16()).toEqual('0x0');
    });
  });

  describe('inOptions', () => {
    ['data' as keyof Options].forEach(input => {
      it(`converts ${input} to hex data`, () => {
        const block: Options = {};

        block[input] = '1234';
        const formatted = inData(block[input]);

        expect(formatted).toEqual('0x1234');
      });
    });

    (['from', 'to'] as (keyof Options)[]).forEach(input => {
      it(`formats ${input} address as address`, () => {
        const block: Options = {};

        block[input] = address;
        const formatted = inOptions(block)[input] as string;

        expect(isAddress(formatted)).toBe(true);
        expect(formatted).toEqual(address);
      });
    });

    it('does not encode an empty `to` value', () => {
      const options = { to: '' };
      const formatted = inOptions(options);

      expect(formatted.to).toEqual(undefined);
    });

    (['gas', 'gasPrice', 'value', 'nonce'] as (keyof Options)[]).forEach(
      input => {
        it(`formats ${input} number as hexnumber`, () => {
          const block: Options = {};

          block[input] = 0x123;
          const formatted = inOptions(block)[input];

          expect(formatted).toEqual('0x123');
        });
      }
    );

    it('passes condition as null when specified as such', () => {
      expect(inOptions({ condition: null })).toEqual({ condition: null });
    });

    it('ignores and passes through unknown keys', () => {
      expect(inOptions({ someRandom: 'someRandom' } as any)).toEqual({
        someRandom: 'someRandom'
      });
    });

    it('formats an options object with relevant entries converted', () => {
      expect(
        inOptions({
          from: address,
          to: address,
          gas: new BigNumber('0x100'),
          gasPrice: 0x101,
          value: 258,
          nonce: '0x104',
          data: '0123456789',
          extraData: 'someExtraStuffInHere'
        })
      ).toEqual({
        from: address,
        to: address,
        gas: '0x100',
        gasPrice: '0x101',
        value: '0x102',
        nonce: '0x104',
        data: '0x0123456789',
        extraData: 'someExtraStuffInHere'
      });
    });
  });

  describe('inTraceType', () => {
    it('returns array of types as is', () => {
      const types = ['vmTrace', 'trace', 'stateDiff'];

      expect(inTraceType(types)).toEqual(types);
    });

    it('formats single string type into array', () => {
      const type = 'vmTrace';

      expect(inTraceType(type)).toEqual([type]);
    });
  });

  describe('inDeriveHash', () => {
    it('returns derive hash', () => {
      expect(inDeriveHash(1)).toEqual({
        hash: '0x1',
        type: 'soft'
      });

      expect(inDeriveHash(null)).toEqual({
        hash: '0x',
        type: 'soft'
      });

      expect(
        inDeriveHash({
          hash: 5
        })
      ).toEqual({
        hash: '0x5',
        type: 'soft'
      });

      expect(
        inDeriveHash({
          hash: 5,
          type: 'hard'
        })
      ).toEqual({
        hash: '0x5',
        type: 'hard'
      });
    });
  });

  describe('inDeriveIndex', () => {
    it('returns derive hash', () => {
      expect(inDeriveIndex(null)).toEqual([]);
      expect(inDeriveIndex([])).toEqual([]);

      expect(inDeriveIndex([1])).toEqual([
        {
          index: 1,
          type: 'soft'
        }
      ]);

      expect(
        inDeriveIndex({
          index: 1
        })
      ).toEqual([
        {
          index: 1,
          type: 'soft'
        }
      ]);

      expect(
        inDeriveIndex([
          {
            index: 1,
            type: 'hard'
          },
          5
        ])
      ).toEqual([
        {
          index: 1,
          type: 'hard'
        },
        {
          index: 5,
          type: 'soft'
        }
      ]);
    });
  });

  describe('inTopics', () => {
    it('returns empty array when no inputs provided', () => {
      expect(inTopics()).toEqual([]);
    });

    it('keeps null topic as null', () => {
      expect(inTopics([null])).toEqual([null]);
    });

    it('pads topics as received', () => {
      expect(inTopics(['123'])).toEqual([
        '0x0000000000000000000000000000000000000000000000000000000000000123'
      ]);
    });

    it('handles nested arrays', () => {
      expect(inTopics([null, '123', ['456', null, '789']])).toEqual([
        null,
        '0x0000000000000000000000000000000000000000000000000000000000000123',
        [
          '0x0000000000000000000000000000000000000000000000000000000000000456',
          null,
          '0x0000000000000000000000000000000000000000000000000000000000000789'
        ]
      ]);
    });
  });
});
