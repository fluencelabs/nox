// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import {
  isArray,
  isError,
  isFunction,
  isHex,
  isInstanceOf,
  isObject,
  isString
} from './types';
import Token from '../token';

describe('util/types', () => {
  describe('isArray', () => {
    it('correctly identifies null as false', () => {
      expect(isArray(null)).toBe(false);
    });

    it('correctly identifies empty arrays as Array', () => {
      expect(isArray([])).toBe(true);
    });

    it('correctly identifies non-empty arrays as Array', () => {
      expect(isArray([1, 2, 3])).toBe(true);
    });

    it('correctly identifies strings as non-Array', () => {
      expect(isArray('not an array')).toBe(false);
    });

    it('correctly identifies objects as non-Array', () => {
      expect(isArray({})).toBe(false);
    });
  });

  describe('isString', () => {
    it('correctly identifies empty string as string', () => {
      expect(isString('')).toBe(true);
    });

    it('correctly identifies string as string', () => {
      expect(isString('123')).toBe(true);
    });
  });

  describe('isInstanceOf', () => {
    it('correctly identifies build-in instanceof', () => {
      expect(isInstanceOf(new String('123'), String)).toBe(true);
    });

    it('correctly identifies own instanceof', () => {
      expect(isInstanceOf(new Token('int', 123), Token)).toBe(true);
    });

    it('correctly reports false for own', () => {
      expect(isInstanceOf({ type: 'int' }, Token)).toBe(false);
    });
  });

  describe('isError', () => {
    it('correctly identifies null as false', () => {
      expect(isError(null)).toBe(false);
    });

    it('correctly identifies Error as true', () => {
      expect(isError(new Error('an error'))).toBe(true);
    });
  });

  describe('isFunction', () => {
    it('correctly identifies null as false', () => {
      expect(isFunction(null)).toBe(false);
    });

    it('correctly identifies function as true', () => {
      expect(isFunction(jest.fn())).toBe(true);
    });
  });

  describe('isHex', () => {
    it('correctly identifies hex by leading 0x', () => {
      expect(isHex('0x123')).toBe(true);
    });

    it('correctly identifies hex without leading 0x', () => {
      expect(isHex('123')).toBe(true);
    });

    it('correctly identifies non-hex values', () => {
      expect(isHex('123j')).toBe(false);
    });

    it('correctly indentifies non-string values', () => {
      expect(isHex(false)).toBe(false);
      expect(isHex(undefined)).toBe(false);
      expect(isHex([1, 2, 3])).toBe(false);
    });
  });

  describe('isObject', () => {
    it('correctly identifies empty object as object', () => {
      expect(isObject({})).toBe(true);
    });

    it('correctly identifies non-empty object as object', () => {
      expect(isObject({ data: '123' })).toBe(true);
    });

    it('correctly identifies Arrays as non-objects', () => {
      expect(isObject([1, 2, 3])).toBe(false);
    });

    it('correctly identifies Strings as non-objects', () => {
      expect(isObject('123')).toBe(false);
    });
  });
});
