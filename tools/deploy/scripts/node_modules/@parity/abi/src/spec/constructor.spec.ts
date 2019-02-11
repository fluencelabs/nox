// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { AbiInput, AbiItem } from '../types';
import Constructor from './constructor';
import Param from './param';
import Token from '../token';

describe('spec/Constructor', () => {
  const inputsArr: AbiInput[] = [
    { name: 'boolin', type: 'bool' },
    { name: 'stringin', type: 'string' }
  ];
  const bool = new Param('boolin', 'bool');
  const baseString = new Param('stringin', 'string');

  const inputs = [bool, baseString];
  const cr = new Constructor({ inputs: inputsArr, type: 'constructor' });

  describe('constructor', () => {
    it('stores the inputs as received', () => {
      expect(cr.inputs).toEqual(inputs);
    });

    it('matches empty inputs with []', () => {
      expect(new Constructor({} as AbiItem).inputs).toEqual([]);
    });
  });

  describe('inputParamTypes', () => {
    it('retrieves the input types as received', () => {
      expect(cr.inputParamTypes()).toEqual([bool.kind, baseString.kind]);
    });
  });

  describe('encodeCall', () => {
    it('encodes correctly', () => {
      const result = cr.encodeCall([
        new Token('bool', true),
        new Token('string', 'jacogr')
      ]);

      expect(result).toEqual(
        '0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000066a61636f67720000000000000000000000000000000000000000000000000000'
      );
    });
  });
});
