// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { TokenTypeEnum } from '../types';
import Token from './token';

describe('token/token', () => {
  describe('validateType', () => {
    it('validates address', () => {
      expect(Token.validateType('address')).toBe(true);
    });

    it('validates fixedArray', () => {
      expect(Token.validateType('fixedArray')).toBe(true);
    });

    it('validates array', () => {
      expect(Token.validateType('array')).toBe(true);
    });

    it('validates fixedBytes', () => {
      expect(Token.validateType('fixedBytes')).toBe(true);
    });

    it('validates bytes', () => {
      expect(Token.validateType('bytes')).toBe(true);
    });

    it('validates bool', () => {
      expect(Token.validateType('bool')).toBe(true);
    });

    it('validates int', () => {
      expect(Token.validateType('int')).toBe(true);
    });

    it('validates uint', () => {
      expect(Token.validateType('uint')).toBe(true);
    });

    it('validates string', () => {
      expect(Token.validateType('string')).toBe(true);
    });

    it('throws an error on invalid types', () => {
      expect(() => Token.validateType('noMatch' as TokenTypeEnum)).toThrow(
        /noMatch/
      );
    });
  });

  describe('constructor', () => {
    it('throws an error on invalid types', () => {
      expect(() => new Token('noMatch' as TokenTypeEnum, '1')).toThrow(
        /noMatch/
      );
    });

    it('sets the type of the object', () => {
      expect(new Token('bool', '1').type).toEqual('bool');
    });

    it('sets the value of the object', () => {
      expect(new Token('bool', '1').value).toEqual('1');
    });
  });
});
