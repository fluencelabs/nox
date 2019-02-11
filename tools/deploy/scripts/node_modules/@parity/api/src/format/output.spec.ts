// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import BigNumber from 'bignumber.js';
import { isAddress } from '@parity/abi/lib/util/address';
import { isInstanceOf } from '@parity/abi/lib/util/types';

import { Block, Receipt } from '../types';
import {
  outBlock,
  outAccountInfo,
  outAddress,
  outChainStatus,
  outDate,
  outHistogram,
  outHwAccountInfo,
  outNodeKind,
  outNumber,
  outPeer,
  outPeers,
  outReceipt,
  outSyncing,
  outTransaction,
  outTrace,
  outVaultMeta
} from './output';
import {
  SerializedBlock,
  SerializedReceipt,
  SerializedTransaction
} from './types.serialized';

describe('format/output', () => {
  const address = '0x63cf90d3f0410092fc0fca41846f596223979195';
  const checksum = '0x63Cf90D3f0410092FC0fca41846f596223979195';

  describe('outAccountInfo', () => {
    it('returns meta objects parsed', () => {
      expect(
        outAccountInfo({
          '0x63cf90d3f0410092fc0fca41846f596223979195': {
            name: 'name',
            uuid: 'uuid',
            meta: '{"name":"456"}'
          }
        })
      ).toEqual({
        '0x63Cf90D3f0410092FC0fca41846f596223979195': {
          name: 'name',
          uuid: 'uuid',
          meta: { name: '456' }
        }
      });
    });

    it('returns objects without meta & uuid as required', () => {
      expect(
        outAccountInfo({
          '0x63cf90d3f0410092fc0fca41846f596223979195': { name: 'name' }
        })
      ).toEqual({
        '0x63Cf90D3f0410092FC0fca41846f596223979195': { name: 'name' }
      });
    });
  });

  describe('outAddress', () => {
    it('retuns the address as checksummed', () => {
      expect(outAddress(address)).toEqual(checksum);
    });

    it('retuns the checksum as checksummed', () => {
      expect(outAddress(checksum)).toEqual(checksum);
    });
  });

  describe('outBlock', () => {
    ['author', 'miner'].forEach(input => {
      it(`formats ${input} address as address`, () => {
        const block: SerializedBlock = {};

        block[input as keyof SerializedBlock] = address;
        const formatted = outBlock(block)[input as keyof Block];

        expect(isAddress(formatted as string)).toBe(true);
        expect(formatted).toEqual(checksum);
      });
    });

    [
      'difficulty',
      'gasLimit',
      'gasUsed',
      'number',
      'nonce',
      'totalDifficulty'
    ].forEach(input => {
      it(`formats ${input} number as hexnumber`, () => {
        const block: SerializedBlock = {};

        block[input as keyof SerializedBlock] = 0x123;
        const formatted = outBlock(block)[input as keyof Block];

        expect(isInstanceOf(formatted, BigNumber)).toBe(true);
        // @ts-ignore
        expect(formatted.toString(16)).toEqual('123');
      });
    });

    ['timestamp'].forEach(input => {
      it(`formats ${input} number as Date`, () => {
        const block: SerializedBlock = {};

        block[input as keyof SerializedBlock] = 0x57513668;
        const formatted = outBlock(block)[input as keyof Block];

        expect(isInstanceOf(formatted, Date)).toBe(true);
        expect((formatted as Date).getTime()).toEqual(1464940136000);
      });
    });

    it('ignores and passes through unknown keys', () => {
      expect(outBlock({ someRandom: 'someRandom' } as any)).toEqual({
        someRandom: 'someRandom'
      });
    });

    it('formats a block with all the info converted', () => {
      expect(
        outBlock({
          author: address,
          miner: address,
          difficulty: '0x100',
          gasLimit: '0x101',
          gasUsed: '0x102',
          number: '0x103',
          nonce: '0x104',
          totalDifficulty: '0x105',
          timestamp: '0x57513668',
          extraData: 'someExtraStuffInHere'
        })
      ).toEqual({
        author: checksum,
        miner: checksum,
        difficulty: new BigNumber('0x100'),
        gasLimit: new BigNumber('0x101'),
        gasUsed: new BigNumber('0x102'),
        number: new BigNumber('0x103'),
        nonce: new BigNumber('0x104'),
        totalDifficulty: new BigNumber('0x105'),
        timestamp: new Date('2016-06-03T07:48:56.000Z'),
        extraData: 'someExtraStuffInHere'
      });
    });
  });

  describe('outChainStatus', () => {
    it('formats blockGap values', () => {
      const status = {
        blockGap: [0x1234, '0x5678']
      };

      expect(outChainStatus(status)).toEqual({
        blockGap: [new BigNumber(0x1234), new BigNumber(0x5678)]
      });
    });

    it('handles null blockGap values', () => {
      const status = {
        blockGap: undefined
      };

      expect(outChainStatus(status)).toEqual(status);
    });
  });

  describe('outDate', () => {
    it('converts a second date in unix timestamp', () => {
      expect(outDate(0x57513668)).toEqual(new Date('2016-06-03T07:48:56.000Z'));
    });
  });

  describe('outHistogram', () => {
    ['bucketBounds', 'counts'].forEach(type => {
      it(`formats ${type} as number arrays`, () => {
        expect(outHistogram({ [type]: [0x123, 0x456, 0x789] })).toEqual({
          [type]: [
            new BigNumber(0x123),
            new BigNumber(0x456),
            new BigNumber(0x789)
          ]
        });
      });
    });
  });

  describe('outHwAccountInfo', () => {
    it('returns objects with formatted addresses', () => {
      expect(
        outHwAccountInfo({
          '0x63cf90d3f0410092fc0fca41846f596223979195': {
            manufacturer: 'mfg',
            name: 'type'
          }
        })
      ).toEqual({
        '0x63Cf90D3f0410092FC0fca41846f596223979195': {
          manufacturer: 'mfg',
          name: 'type'
        }
      });
    });
  });

  describe('outNodeKind', () => {
    it('formats the input as received', () => {
      const kind = { availability: 'personal', capability: 'full' };

      expect(outNodeKind(kind)).toEqual(kind);
    });
  });

  describe('outNumber', () => {
    it('returns a BigNumber equalling the value', () => {
      const bn = outNumber('0x123456');

      expect(isInstanceOf(bn, BigNumber)).toBe(true);
      expect(bn.eq(0x123456)).toBe(true);
    });

    it('assumes 0 when ivalid input', () => {
      expect(outNumber().eq(0)).toBe(true);
    });
  });

  describe('outPeer', () => {
    it('converts all internal numbers to BigNumbers', () => {
      expect(
        outPeer({
          caps: ['par/1'],
          id: '0x01',
          name: 'Parity',
          network: {
            localAddress: '10.0.0.1',
            remoteAddress: '10.0.0.1'
          },
          protocols: {
            par: {
              difficulty: '0x0f',
              head: '0x02',
              version: 63
            }
          }
        })
      ).toEqual({
        caps: ['par/1'],
        id: '0x01',
        name: 'Parity',
        network: {
          localAddress: '10.0.0.1',
          remoteAddress: '10.0.0.1'
        },
        protocols: {
          par: {
            difficulty: new BigNumber(15),
            head: '0x02',
            version: 63
          }
        }
      });
    });

    it('does not output null protocols', () => {
      expect(
        outPeer({
          caps: ['par/1'],
          id: '0x01',
          name: 'Parity',
          network: {
            localAddress: '10.0.0.1',
            remoteAddress: '10.0.0.1'
          },
          protocols: {
            les: null
          }
        })
      ).toEqual({
        caps: ['par/1'],
        id: '0x01',
        name: 'Parity',
        network: {
          localAddress: '10.0.0.1',
          remoteAddress: '10.0.0.1'
        },
        protocols: {}
      });
    });
  });

  describe('outPeers', () => {
    it('converts all internal numbers to BigNumbers', () => {
      expect(
        outPeers({
          active: 789,
          connected: '456',
          max: 0x7b,
          peers: [
            {
              caps: ['par/1'],
              id: '0x01',
              name: 'Parity',
              network: {
                localAddress: '10.0.0.1',
                remoteAddress: '10.0.0.1'
              },
              protocols: {
                par: {
                  difficulty: '0x0f',
                  head: '0x02',
                  version: 63
                },
                les: null
              }
            }
          ]
        })
      ).toEqual({
        active: new BigNumber(789),
        connected: new BigNumber(456),
        max: new BigNumber(123),
        peers: [
          {
            caps: ['par/1'],
            id: '0x01',
            name: 'Parity',
            network: {
              localAddress: '10.0.0.1',
              remoteAddress: '10.0.0.1'
            },
            protocols: {
              par: {
                difficulty: new BigNumber(15),
                head: '0x02',
                version: 63
              }
            }
          }
        ]
      });
    });
  });

  describe('outReceipt', () => {
    ['contractAddress'].forEach(input => {
      it(`formats ${input} address as address`, () => {
        const block: SerializedReceipt = {};

        block[input as keyof SerializedReceipt] = address;
        const formatted = outReceipt(block)[input as keyof Receipt];

        expect(isAddress(formatted as string)).toBe(true);
        expect(formatted).toEqual(checksum);
      });
    });

    [
      'blockNumber',
      'cumulativeGasUsed',
      'cumulativeGasUsed',
      'gasUsed',
      'transactionIndex'
    ].forEach(input => {
      it(`formats ${input} number as hexnumber`, () => {
        const block: SerializedReceipt = {};

        block[input as keyof SerializedReceipt] = 0x123;
        const formatted = outReceipt(block)[input as keyof Receipt];

        expect(isInstanceOf(formatted, BigNumber)).toBe(true);
        // @ts-ignore
        expect(formatted.toString(16)).toEqual('123');
      });
    });

    it('ignores and passes through unknown keys', () => {
      // @ts-ignore
      expect(outReceipt({ someRandom: 'someRandom' })).toEqual({
        someRandom: 'someRandom'
      });
    });

    it('formats a receipt with all the info converted', () => {
      expect(
        outReceipt({
          contractAddress: address,
          blockNumber: '0x100',
          cumulativeGasUsed: '0x101',
          gasUsed: '0x102',
          transactionIndex: '0x103',
          extraData: 'someExtraStuffInHere'
        })
      ).toEqual({
        contractAddress: checksum,
        blockNumber: new BigNumber('0x100'),
        cumulativeGasUsed: new BigNumber('0x101'),
        gasUsed: new BigNumber('0x102'),
        transactionIndex: new BigNumber('0x103'),
        extraData: 'someExtraStuffInHere'
      });
    });
  });

  describe('outSyncing', () => {
    [
      'currentBlock',
      'highestBlock',
      'startingBlock',
      'warpChunksAmount',
      'warpChunksProcessed'
    ].forEach(input => {
      it(`formats ${input} numbers as a number`, () => {
        expect(outSyncing({ [input]: '0x123' })).toEqual({
          [input]: new BigNumber('0x123')
        });
      });
    });

    it('formats blockGap properly', () => {
      expect(outSyncing({ blockGap: [0x123, 0x456] })).toEqual({
        blockGap: [new BigNumber(0x123), new BigNumber(0x456)]
      });
    });
  });

  describe('outTransaction', () => {
    ['from', 'to'].forEach(input => {
      it(`formats ${input} address as address`, () => {
        const block: SerializedTransaction = {};

        block[input as keyof SerializedTransaction] = address;
        const formatted = outTransaction(block)[
          input as keyof SerializedTransaction
        ];

        expect(isAddress(formatted as string)).toBe(true);
        expect(formatted).toEqual(checksum);
      });
    });

    [
      'blockNumber',
      'gasPrice',
      'gas',
      'nonce',
      'transactionIndex',
      'value'
    ].forEach(input => {
      it(`formats ${input} number as hexnumber`, () => {
        const block: SerializedTransaction = {};

        block[input as keyof SerializedTransaction] = 0x123;
        const formatted = outTransaction(block)[
          input as keyof SerializedTransaction
        ];

        expect(isInstanceOf(formatted, BigNumber)).toBe(true);
        // @ts-ignore
        expect(formatted.toString(16)).toEqual('123');
      });
    });

    it('passes condition as null when null', () => {
      expect(outTransaction({ condition: null })).toEqual({ condition: null });
    });

    it('ignores and passes through unknown keys', () => {
      // @ts-ignore
      expect(outTransaction({ someRandom: 'someRandom' })).toEqual({
        someRandom: 'someRandom'
      });
    });

    it('formats a transaction with all the info converted', () => {
      expect(
        outTransaction({
          from: address,
          to: address,
          blockNumber: '0x100',
          gasPrice: '0x101',
          gas: '0x102',
          nonce: '0x103',
          transactionIndex: '0x104',
          value: '0x105',
          extraData: 'someExtraStuffInHere'
        })
      ).toEqual({
        from: checksum,
        to: checksum,
        blockNumber: new BigNumber('0x100'),
        gasPrice: new BigNumber('0x101'),
        gas: new BigNumber('0x102'),
        nonce: new BigNumber('0x103'),
        transactionIndex: new BigNumber('0x104'),
        value: new BigNumber('0x105'),
        extraData: 'someExtraStuffInHere'
      });
    });
  });

  describe('outTrace', () => {
    it('ignores and passes through unknown keys', () => {
      // @ts-ignore
      expect(outTrace({ someRandom: 'someRandom' })).toEqual({
        someRandom: 'someRandom'
      });
    });

    it('formats a trace with all the info converted', () => {
      const formatted = outTrace({
        type: 'call',
        action: {
          from: address,
          to: address,
          value: '0x06',
          gas: '0x07',
          input: '0x1234',
          callType: 'call'
        },
        result: {
          gasUsed: '0x08',
          output: '0x5678'
        },
        traceAddress: ['0x2'],
        subtraces: 3,
        transactionPosition: '0xb',
        transactionHash:
          '0x000000000000000000000000000000000000000000000000000000000000000c',
        blockNumber: '0x0d',
        blockHash:
          '0x000000000000000000000000000000000000000000000000000000000000000e'
      });

      // @ts-ignore
      expect(isInstanceOf(formatted.action.gas, BigNumber)).toBe(true);
      // @ts-ignore
      expect(formatted.action.gas.toNumber()).toEqual(7);
      // @ts-ignore
      expect(isInstanceOf(formatted.action.value, BigNumber)).toBe(true);
      // @ts-ignore
      expect(formatted.action.value.toNumber()).toEqual(6);

      // @ts-ignore
      expect(formatted.action.from).toEqual(checksum);
      // @ts-ignore
      expect(formatted.action.to).toEqual(checksum);

      // @ts-ignore
      expect(isInstanceOf(formatted.blockNumber, BigNumber)).toBe(true);
      // @ts-ignore
      expect(formatted.blockNumber.toNumber()).toEqual(13);
      // @ts-ignore
      expect(isInstanceOf(formatted.transactionPosition, BigNumber)).toBe(true);
      // @ts-ignore
      expect(formatted.transactionPosition.toNumber()).toEqual(11);
    });
  });

  describe('outVaultMeta', () => {
    it('returns an exmpt object on null', () => {
      expect(outVaultMeta(null)).toEqual({});
    });

    it('returns the original value if not string', () => {
      expect(outVaultMeta({ test: 123 })).toEqual({ test: 123 });
    });

    it('returns an object from JSON string', () => {
      expect(outVaultMeta('{"test":123}')).toEqual({ test: 123 });
    });

    it('returns an empty object on invalid JSON', () => {
      expect(outVaultMeta('{"test"}')).toEqual({});
    });
  });
});
