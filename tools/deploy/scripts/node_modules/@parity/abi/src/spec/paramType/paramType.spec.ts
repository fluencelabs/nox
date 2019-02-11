// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import ParamType from './paramType';
import { TokenTypeEnum } from '../../types';

describe('spec/paramType/ParamType', () => {
  describe('validateType', () => {
    it('validates address', () => {
      expect(ParamType.validateType('address')).toBe(true);
    });

    it('validates fixedArray', () => {
      expect(ParamType.validateType('fixedArray')).toBe(true);
    });

    it('validates array', () => {
      expect(ParamType.validateType('array')).toBe(true);
    });

    it('validates fixedBytes', () => {
      expect(ParamType.validateType('fixedBytes')).toBe(true);
    });

    it('validates bytes', () => {
      expect(ParamType.validateType('bytes')).toBe(true);
    });

    it('validates bool', () => {
      expect(ParamType.validateType('bool')).toBe(true);
    });

    it('validates int', () => {
      expect(ParamType.validateType('int')).toBe(true);
    });

    it('validates uint', () => {
      expect(ParamType.validateType('uint')).toBe(true);
    });

    it('validates string', () => {
      expect(ParamType.validateType('string')).toBe(true);
    });

    it('throws an error on invalid types', () => {
      expect(() => ParamType.validateType('noMatch' as TokenTypeEnum)).toThrow(
        /noMatch/
      );
    });
  });

  describe('constructor', () => {
    it('throws an error on invalid types', () => {
      expect(() => new ParamType('noMatch' as TokenTypeEnum)).toThrow(
        /noMatch/
      );
    });

    it('sets the type of the object', () => {
      expect(new ParamType('bool', undefined, 1).type).toEqual('bool');
    });

    it('sets the subtype of the object', () => {
      expect(new ParamType('array', new ParamType('bool'), 1).subtype).toEqual({
        _indexed: false,
        _length: 0,
        _subtype: undefined,
        _type: 'bool'
      });
    });

    it('sets the length of the object', () => {
      expect(new ParamType('array', new ParamType('bool'), 1).length).toEqual(
        1
      );
    });

    it('sets the index of the object', () => {
      expect(
        new ParamType('array', new ParamType('bool'), 1, true).indexed
      ).toBe(true);
    });

    it('sets default values where none supplied', () => {
      expect(Object.values(new ParamType('string'))).toEqual([
        'string',
        undefined,
        0,
        false
      ]);
    });
  });
});
