// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import * as utf8 from 'utf8';

import Token from '../token/token';
import BytesTaken from './bytesTaken';
import DecodeResult from './decodeResult';
import ParamType from '../spec/paramType/paramType';
import { sliceData } from '../util/slice';
import { asAddress, asBool, asI32, asU32 } from '../util/sliceAs';
import { isArray, isInstanceOf } from '../util/types';
import { Slices } from '../types';

const NULL = '0000000000000000000000000000000000000000000000000000000000000000';

class Decoder {
  static decode (params: ParamType[] | undefined, data?: string) {
    if (!isArray(params)) {
      throw new Error('Parameters should be array of ParamType');
    }

    const slices = sliceData(data);
    let offset = 0;

    return params.map(param => {
      const result = Decoder.decodeParam(param, slices, offset);

      offset = result.newOffset;
      return result.token;
    });
  }

  static peek (slices: Slices, position: number) {
    if (!slices || !slices[position]) {
      return NULL;
    }

    return slices[position];
  }

  static takeBytes (slices: Slices, position: number, length: number) {
    const slicesLength = Math.floor((length + 31) / 32);
    let bytesStr = '';

    for (let index = 0; index < slicesLength; index++) {
      bytesStr = `${bytesStr}${Decoder.peek(slices, position + index)}`;
    }

    const bytes = (bytesStr.substr(0, length * 2).match(/.{1,2}/g) || []).map(
      code => parseInt(code, 16)
    );

    return new BytesTaken(bytes, position + slicesLength);
  }

  static decodeParam (
    param: ParamType,
    slices: Slices,
    offset: number = 0
  ): DecodeResult {
    if (!isInstanceOf(param, ParamType)) {
      throw new Error('param should be instanceof ParamType');
    }

    const tokens = [];
    let taken;
    let lengthOffset;
    let length;
    let newOffset;

    switch (param.type) {
      case 'address':
        return new DecodeResult(
          new Token(param.type, asAddress(Decoder.peek(slices, offset))),
          offset + 1
        );

      case 'bool':
        return new DecodeResult(
          new Token(param.type, asBool(Decoder.peek(slices, offset))),
          offset + 1
        );

      case 'int':
        return new DecodeResult(
          new Token(param.type, asI32(Decoder.peek(slices, offset))),
          offset + 1
        );

      case 'uint':
        return new DecodeResult(
          new Token(param.type, asU32(Decoder.peek(slices, offset))),
          offset + 1
        );

      case 'fixedBytes': {
        taken = Decoder.takeBytes(slices, offset, param.length);

        return new DecodeResult(
          new Token(param.type, taken.bytes),
          taken.newOffset
        );
      }
      case 'bytes': {
        lengthOffset = asU32(Decoder.peek(slices, offset))
          .div(32)
          .toNumber();
        length = asU32(Decoder.peek(slices, lengthOffset)).toNumber();
        taken = Decoder.takeBytes(slices, lengthOffset + 1, length);

        return new DecodeResult(new Token(param.type, taken.bytes), offset + 1);
      }
      case 'string': {
        if (param.indexed) {
          taken = Decoder.takeBytes(slices, offset, 32);

          return new DecodeResult(
            new Token('fixedBytes', taken.bytes),
            offset + 1
          );
        }

        lengthOffset = asU32(Decoder.peek(slices, offset))
          .div(32)
          .toNumber();
        length = asU32(Decoder.peek(slices, lengthOffset)).toNumber();
        taken = Decoder.takeBytes(slices, lengthOffset + 1, length);

        const str = taken.bytes
          .map((code: number) => String.fromCharCode(code))
          .join('');

        let decoded;

        try {
          decoded = utf8.decode(str);
        } catch (error) {
          decoded = str;
        }

        return new DecodeResult(new Token(param.type, decoded), offset + 1);
      }
      case 'array': {
        if (!param.subtype) {
          throw new Error(
            `decodeParam: param of type '${param.type}' must have a subtype`
          );
        }

        lengthOffset = asU32(Decoder.peek(slices, offset))
          .div(32)
          .toNumber();
        length = asU32(Decoder.peek(slices, lengthOffset)).toNumber();
        newOffset = lengthOffset + 1;

        for (let index = 0; index < length; index++) {
          const result = Decoder.decodeParam(param.subtype, slices, newOffset);

          newOffset = result.newOffset;
          tokens.push(result.token);
        }

        return new DecodeResult(new Token(param.type, tokens), offset + 1);
      }
      case 'fixedArray': {
        if (!param.subtype) {
          throw new Error(
            `decodeParam: param of type '${param.type}' must have a subtype`
          );
        }

        newOffset = offset;

        for (let index = 0; index < param.length; index++) {
          const result = Decoder.decodeParam(param.subtype, slices, newOffset);

          newOffset = result.newOffset;
          tokens.push(result.token);
        }

        return new DecodeResult(new Token(param.type, tokens), newOffset);
      }
      default:
        throw new Error(`Invalid param type ${param.type} in decodeParam`);
    }
  }
}

export default Decoder;
