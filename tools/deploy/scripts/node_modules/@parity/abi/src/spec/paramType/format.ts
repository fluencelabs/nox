// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import ParamType from './paramType';
import { TokenTypeEnum } from '../../types';

/**
 * Convert a string to a ParamType.
 *
 * @param type - Type to convert.
 * @param indexed - Whether the ParamType is indexed or not.
 */
export const toParamType = (type: string, indexed?: boolean): ParamType => {
  if (type[type.length - 1] === ']') {
    const last = type.lastIndexOf('[');
    const length = type.substr(last + 1, type.length - last - 2);
    const subtype = toParamType(type.substr(0, last) as TokenTypeEnum);

    if (length.length === 0) {
      return new ParamType('array', subtype, 0, indexed);
    }

    return new ParamType('fixedArray', subtype, parseInt(length, 10), indexed);
  }

  switch (type) {
    case 'address':
    case 'bool':
    case 'bytes':
    case 'string':
      return new ParamType(type, undefined, 0, indexed);

    case 'int':
    case 'uint':
      return new ParamType(type, undefined, 256, indexed);

    default:
      if (type.indexOf('uint') === 0) {
        return new ParamType(
          'uint',
          undefined,
          parseInt(type.substr(4), 10),
          indexed
        );
      } else if (type.indexOf('int') === 0) {
        return new ParamType(
          'int',
          undefined,
          parseInt(type.substr(3), 10),
          indexed
        );
      } else if (type.indexOf('bytes') === 0) {
        return new ParamType(
          'fixedBytes',
          undefined,
          parseInt(type.substr(5), 10),
          indexed
        );
      }

      throw new Error(`Cannot convert ${type} to valid ParamType`);
  }
};

/**
 * Convert a ParamType to its string representation.
 *
 * @param paramType - ParamType instance to convert
 */
export const fromParamType = (paramType: ParamType): string => {
  switch (paramType.type) {
    case 'address':
    case 'bool':
    case 'bytes':
    case 'string':
      return paramType.type;

    case 'int':
    case 'uint':
      return `${paramType.type}${paramType.length}`;

    case 'fixedBytes':
      return `bytes${paramType.length}`;

    case 'fixedArray': {
      if (!paramType.subtype) {
        throw new Error(
          `decodeParam: param of type '${paramType.type}' must have a subtype`
        );
      }
      return `${fromParamType(paramType.subtype)}[${paramType.length}]`;
    }

    case 'array': {
      if (!paramType.subtype) {
        throw new Error(
          `decodeParam: param of type '${paramType.type}' must have a subtype`
        );
      }
      return `${fromParamType(paramType.subtype)}[]`;
    }

    default:
      throw new Error(`Cannot convert from ParamType ${paramType.type}`);
  }
};

module.exports = {
  fromParamType,
  toParamType
};
