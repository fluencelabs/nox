// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { AbiItem, TokenTypeEnum, TokenValue } from '@parity/abi';
import {
  fromParamType,
  toParamType
} from '@parity/abi/lib/spec/paramType/format';
import Func from '@parity/abi/lib/spec/function';

import { isHex } from './types';

/**
 * Decode call data.
 *
 * @param data - The call data to decode.
 */
export const decodeCallData = (
  data: string
): { paramdata: string; signature: string } => {
  if (!isHex(data)) {
    throw new Error('Input to decodeCallData should be a hex value');
  }

  if (data.startsWith('0x')) {
    return decodeCallData(data.slice(2));
  }

  if (data.length < 8) {
    throw new Error(
      'Input to decodeCallData should be method signature + data'
    );
  }

  const signature = data.substr(0, 8);
  const paramdata = data.substr(8);

  return {
    signature: `0x${signature}`,
    paramdata: `0x${paramdata}`
  };
};

/**
 * Decode the method inputs.
 *
 * @param methodAbi - The ABI of the method.
 * @param paramdata -
 */
export const decodeMethodInput = (
  methodAbi: AbiItem,
  paramdata?: string
): TokenValue[] => {
  if (!methodAbi) {
    throw new Error(
      'decodeMethodInput should receive valid method-specific ABI'
    );
  }

  if (paramdata && paramdata.length) {
    if (!isHex(paramdata)) {
      throw new Error('Input to decodeMethodInput should be a hex value');
    }

    if (paramdata.substr(0, 2) === '0x') {
      return decodeMethodInput(methodAbi, paramdata.slice(2));
    }
  }

  return new Func(methodAbi)
    .decodeInput(paramdata)
    .map(decoded => decoded.value) as TokenValue[];
};

/**
 * Takes a method in form name(...,types) and returns the inferred abi definition.
 *
 * @param method - The method to convert to abi.
 */
export const methodToAbi = (method: string) => {
  const length = method.length;
  const typesStart = method.indexOf('(');
  const typesEnd = method.indexOf(')');

  if (typesStart === -1) {
    throw new Error(`Missing start ( in call to decodeMethod with ${method}`);
  } else if (typesEnd === -1) {
    throw new Error(`Missing end ) in call to decodeMethod with ${method}`);
  } else if (typesEnd < typesStart) {
    throw new Error(
      `End ) is before start ( in call to decodeMethod with ${method}`
    );
  } else if (typesEnd !== length - 1) {
    throw new Error(
      `Extra characters after end ) in call to decodeMethod with ${method}`
    );
  }

  const name = method.substr(0, typesStart);
  const types = method
    .substr(typesStart + 1, length - (typesStart + 1) - 1)
    .split(',');
  const inputs = types
    .filter(_type => _type.length)
    .map(_type => {
      const type = fromParamType(toParamType(_type));

      return { type };
    });

  return { type: 'function', name, inputs };
};

/**
 * Decode fata from an ABI.
 *
 * @param inputTypes - The types of the inputs.
 * @param data - The data passed to this ABI.
 */
export const abiDecode = (inputTypes: TokenTypeEnum[], data: string) => {
  return decodeMethodInput(
    {
      inputs: inputTypes.map(type => {
        return { type };
      })
    },
    data
  );
};
