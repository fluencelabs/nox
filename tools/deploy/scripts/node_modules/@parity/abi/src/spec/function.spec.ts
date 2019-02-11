// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { AbiItem, AbiInput } from '../types';
import Func from './function';
import Param from './param';
import Token from '../token';

describe('spec/Function', () => {
  const inputsArr: AbiInput[] = [
    { name: 'boolin', type: 'bool' },
    { name: 'stringin', type: 'string' }
  ];
  const outputsArr: AbiInput[] = [{ name: 'output', type: 'uint' }];

  const uint = new Param('output', 'uint');
  const bool = new Param('boolin', 'bool');
  const baseString = new Param('stringin', 'string');
  const inputs = [bool, baseString];
  const outputs = [uint];

  const func = new Func({
    name: 'test',
    inputs: inputsArr,
    outputs: outputsArr
  } as AbiItem);

  describe('constructor', () => {
    it('returns signature correctly if name already contains it', () => {
      const func = new Func({
        name: 'test(bool,string)',
        inputs: inputsArr,
        outputs: outputsArr
      } as AbiItem);

      expect(func.name).toEqual('test');
      expect(func.id).toEqual('test(bool,string)');
      expect(func.signature).toEqual('02356205');
    });

    it('stores the parameters as received', () => {
      expect(func.name).toEqual('test');
      expect(func.constant).toBe(false);
      expect(func.inputs).toEqual(inputs);
      expect(func.outputs).toEqual(outputs);
    });

    it('matches empty inputs with []', () => {
      expect(
        new Func({ name: 'test', outputs: outputsArr } as AbiItem).inputs
      ).toEqual([]);
    });

    it('matches empty outputs with []', () => {
      expect(
        new Func({ name: 'test', inputs: inputsArr } as AbiItem).outputs
      ).toEqual([]);
    });

    it('sets the method signature', () => {
      expect(new Func({ name: 'baz' } as AbiItem).signature).toEqual(
        'a7916fac'
      );
    });

    it('allows constant functions', () => {
      expect(
        new Func({ name: 'baz', constant: true } as AbiItem).constant
      ).toBe(true);
    });
  });

  describe('getters', () => {
    const abi: AbiItem = {
      name: 'test(bool,string)',
      inputs: inputsArr,
      outputs: outputsArr,
      type: 'function'
    };
    const func = new Func(abi);

    it('returns the abi', () => {
      expect(func.abi).toEqual(abi);
    });

    it('returns the constant flag', () => {
      expect(func.constant).toBe(false);
    });

    it('returns the id', () => {
      expect(func.id).toEqual('test(bool,string)');
    });

    it('returns the inputs', () => {
      expect(func.inputs).toEqual(Param.toParams(inputsArr));
    });

    it('returns the outputs', () => {
      expect(func.outputs).toEqual(Param.toParams(outputsArr));
    });

    it('returns the payable flag', () => {
      expect(func.payable).toBe(false);
    });
  });

  describe('inputParamTypes', () => {
    it('retrieves the input types as received', () => {
      expect(func.inputParamTypes()).toEqual([bool.kind, baseString.kind]);
    });
  });

  describe('outputParamTypes', () => {
    it('retrieves the output types as received', () => {
      expect(func.outputParamTypes()).toEqual([uint.kind]);
    });
  });

  describe('decodeInput', () => {
    it('decodes the inputs correctly', () => {
      expect(
        func.decodeInput(
          '0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000066a61636f67720000000000000000000000000000000000000000000000000000'
        )
      ).toEqual([
        {
          _type: 'bool',
          _value: true
        },
        {
          _type: 'string',
          _value: 'jacogr'
        }
      ]);
    });
  });

  describe('decodeOutput', () => {
    it('decodes the result correctly', () => {
      const result = func.decodeOutput(
        '1111111111111111111111111111111111111111111111111111111111111111'
      );

      // @ts-ignore toString doesn't take any args
      expect(result[0].value.toString(16)).toEqual(
        '1111111111111111111111111111111111111111111111111111111111111111'
      );
    });
  });

  describe('encodeCall', () => {
    it('encodes the call correctly', () => {
      const result = func.encodeCall([
        new Token('bool', true),
        new Token('string', 'jacogr')
      ]);

      expect(result).toEqual(
        '023562050000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000066a61636f67720000000000000000000000000000000000000000000000000000'
      );
    });
  });
});
