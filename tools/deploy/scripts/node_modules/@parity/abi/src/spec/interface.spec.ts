// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { AbiItem, AbiItemType } from '../types';
import Event from './event';
import Func from './function';
import Interface from './interface';
import ParamType from './paramType';
import Token from '../token';

describe('spec/Interface', () => {
  const construct: AbiItem = {
    type: 'constructor',
    inputs: []
  };
  const event: AbiItem = {
    type: 'event',
    name: 'Event2',
    anonymous: false,
    inputs: [
      { name: 'a', type: 'uint256', indexed: true },
      { name: 'b', type: 'bytes32', indexed: false }
    ]
  };
  const func: AbiItem = {
    type: 'function',
    name: 'foo',
    inputs: [{ name: 'a', type: 'uint256' }],
    outputs: []
  };

  describe('parseABI', () => {
    it('throws on invalid types', () => {
      expect(() =>
        Interface.parseABI([{ inputs: [], type: 'noMatch' as AbiItemType }])
      ).toThrow(/noMatch/);
    });

    it('creates constructors', () => {
      expect(Interface.parseABI([construct])).toEqual([{ _inputs: [] }]);
    });

    it('creates events', () => {
      const parsed = Interface.parseABI([event])[0] as Event;
      expect(parsed.name).toEqual('Event2');
    });

    it('creates functions', () => {
      const parsed = Interface.parseABI([func])[0] as Func;
      expect(parsed.name).toEqual('foo');
    });

    it('parse complex interfaces', () => {
      expect(Interface.parseABI([construct, event, func]).length).toEqual(3);
    });
  });

  describe('constructor', () => {
    const int = new Interface([construct, event, func]);

    it('contains the full interface', () => {
      expect(int.interface.length).toEqual(3);
    });

    it('contains the constructors', () => {
      expect(int.constructors.length).toEqual(1);
    });

    it('contains the events', () => {
      expect(int.events.length).toEqual(1);
    });

    it('contains the functions', () => {
      expect(int.functions.length).toEqual(1);
    });
  });

  describe('encodeTokens', () => {
    const int = new Interface([construct, event, func]);

    it('encodes simple types', () => {
      expect(
        int.encodeTokens(
          [
            new ParamType('bool'),
            new ParamType('string'),
            new ParamType('int'),
            new ParamType('uint')
          ],
          [true, 'gavofyork', -123, 123]
        )
      ).toEqual([
        new Token('bool', true),
        new Token('string', 'gavofyork'),
        new Token('int', -123),
        new Token('uint', 123)
      ]);
    });

    it('encodes array', () => {
      expect(
        int.encodeTokens(
          [new ParamType('array', new ParamType('bool'))],
          [[true, false, true]]
        )
      ).toEqual([
        new Token('array', [
          new Token('bool', true),
          new Token('bool', false),
          new Token('bool', true)
        ])
      ]);
    });

    it('encodes simple with array of array', () => {
      expect(
        int.encodeTokens(
          [
            new ParamType('bool'),
            new ParamType(
              'fixedArray',
              new ParamType('array', new ParamType('uint')),
              2
            )
          ],
          [true, [[0, 1], [2, 3]]]
        )
      ).toEqual([
        new Token('bool', true),
        new Token('fixedArray', [
          new Token('array', [new Token('uint', 0), new Token('uint', 1)]),
          new Token('array', [new Token('uint', 2), new Token('uint', 3)])
        ])
      ]);
    });
  });
});
