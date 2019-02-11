// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import {
  padAddress,
  padBool,
  padBytes,
  padFixedBytes,
  padU32,
  padString
} from '../util/pad';
import Mediate from './mediate';
import Token from '../token/token';
import { isArray, isInstanceOf } from '../util/types';
import {
  AddressValue,
  BoolValue,
  BytesValue,
  FixedBytesValue,
  IntValue,
  StringValue
} from '../types';

class Encoder {
  static encode (tokens: Token[]) {
    if (!isArray(tokens)) {
      throw new Error('tokens should be array of Token');
    }

    const mediates = tokens.map((token, index) =>
      Encoder.encodeToken(token, index)
    );
    const inits = mediates
      .map((mediate, index) => mediate.init(Mediate.offsetFor(mediates, index)))
      .join('');
    const closings = mediates
      .map((mediate, index) =>
        mediate.closing(Mediate.offsetFor(mediates, index))
      )
      .join('');

    return `${inits}${closings}`;
  }

  static encodeToken (token: Token, index = 0): Mediate {
    if (!token || !isInstanceOf(token, Token)) {
      throw new Error('token should be instanceof Token');
    }

    try {
      switch (token.type) {
        case 'address':
          return new Mediate('raw', padAddress(token.value as AddressValue));

        case 'int':
        case 'uint':
          return new Mediate('raw', padU32(token.value as IntValue));

        case 'bool':
          return new Mediate('raw', padBool(token.value as BoolValue));

        case 'fixedBytes':
          return new Mediate(
            'raw',
            padFixedBytes(token.value as FixedBytesValue)
          );

        case 'bytes':
          return new Mediate('prefixed', padBytes(token.value as BytesValue));

        case 'string':
          return new Mediate('prefixed', padString(token.value as StringValue));

        case 'fixedArray':
        case 'array':
          return new Mediate(
            token.type,
            // TODO token.value as Token[] seems weird.
            (token.value as Token[]).map(token => Encoder.encodeToken(token))
          );
      }
    } catch (e) {
      throw new Error(
        `Cannot encode token #${index} [${token.type}: ${token.value}]. ${
          e.message
        }`
      );
    }

    throw new Error(`Invalid token type ${token.type} in encodeToken`);
  }
}

export default Encoder;
