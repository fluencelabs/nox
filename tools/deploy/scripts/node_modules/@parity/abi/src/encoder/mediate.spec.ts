// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { MediateType } from '../types';
import Mediate from './mediate';

describe('encoder/Mediate', () => {
  const LONG15 =
    '1234567890abcdef000000000000000000000000000000000000000000000000';
  const DOUBLE15 = `${LONG15}${LONG15}`;
  const ARRAY = [new Mediate('raw', DOUBLE15), new Mediate('raw', LONG15)];

  describe('validateType', () => {
    it('validates raw', () => {
      expect(Mediate.validateType('raw')).toBe(true);
    });

    it('validates prefixed', () => {
      expect(Mediate.validateType('prefixed')).toBe(true);
    });

    it('validates fixedArray', () => {
      expect(Mediate.validateType('fixedArray')).toBe(true);
    });

    it('validates array', () => {
      expect(Mediate.validateType('array')).toBe(true);
    });

    it('throws an error on invalid types', () => {
      expect(() => Mediate.validateType('noMatch' as MediateType)).toThrow(
        /noMatch/
      );
    });
  });

  describe('offsetFor', () => {
    it('thows an error when offset < 0', () => {
      expect(() => Mediate.offsetFor([new Mediate('raw', 1)], -1)).toThrow(
        /Invalid position/
      );
    });

    it('throws an error when offset >= length', () => {
      expect(() => Mediate.offsetFor([new Mediate('raw', 1)], 1)).toThrow(
        /Invalid position/
      );
    });
  });

  describe('constructor', () => {
    it('throws an error on invalid types', () => {
      expect(() => new Mediate('noMatch' as MediateType, '1')).toThrow(
        /noMatch/
      );
    });

    it('sets the type of the object', () => {
      expect(new Mediate('raw', '1').type).toEqual('raw');
    });

    it('sets the value of the object', () => {
      expect(new Mediate('raw', '1').value).toEqual('1');
    });
  });

  describe('initLength', () => {
    it('returns correct variable byte length for raw', () => {
      expect(new Mediate('raw', DOUBLE15).initLength()).toEqual(64);
    });

    it('returns correct fixed byte length for array', () => {
      expect(new Mediate('array', [1, 2, 3, 4]).initLength()).toEqual(32);
    });

    it('returns correct fixed byte length for prefixed', () => {
      expect(new Mediate('prefixed', 0).initLength()).toEqual(32);
    });

    it('returns correct variable byte length for fixedArray', () => {
      expect(new Mediate('fixedArray', ARRAY).initLength()).toEqual(96);
    });
  });

  describe('closingLength', () => {
    it('returns 0 byte length for raw', () => {
      expect(new Mediate('raw', DOUBLE15).closingLength()).toEqual(0);
    });

    it('returns prefix + size for prefixed', () => {
      expect(new Mediate('prefixed', DOUBLE15).closingLength()).toEqual(64);
    });

    it('returns prefix + size for array', () => {
      expect(new Mediate('array', ARRAY).closingLength()).toEqual(128);
    });

    it('returns total length for fixedArray', () => {
      expect(new Mediate('fixedArray', ARRAY).closingLength()).toEqual(96);
    });
  });
});
