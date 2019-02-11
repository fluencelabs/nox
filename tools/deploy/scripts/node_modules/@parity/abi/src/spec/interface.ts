// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { AbiObject, TokenValue } from '../types';
import Constructor from './constructor';
import Event from './event/event';
import Func from './function';
import ParamType from './paramType';
import Token from '../token';

class Interface {
  private _interface: (Constructor | Event | Func)[];

  constructor (abi: AbiObject) {
    this._interface = Interface.parseABI(abi);
  }

  static encodeTokens (paramTypes: ParamType[], values: TokenValue[]) {
    const createToken = (paramType: ParamType, value: TokenValue): Token => {
      if (paramType.subtype) {
        return new Token(
          paramType.type,
          (value as TokenValue[]).map(entry =>
            createToken(paramType.subtype as ParamType, entry)
          )
        );
      }

      return new Token(paramType.type, value);
    };

    return paramTypes.map((paramType, index) =>
      createToken(paramType, values[index])
    );
  }

  static parseABI (abi: AbiObject) {
    return abi.map(item => {
      switch (item.type) {
        case 'constructor':
          return new Constructor(item);

        case 'event':
          return new Event(item);

        case 'function':
        case 'fallback':
          return new Func(item);

        default:
          throw new Error(`Unknown ABI type ${item.type}`);
      }
    });
  }

  get interface () {
    return this._interface;
  }

  get constructors () {
    return this._interface.filter(item => item instanceof Constructor);
  }

  get events () {
    return this._interface.filter(item => item instanceof Event);
  }

  get functions () {
    return this._interface.filter(item => item instanceof Func);
  }

  encodeTokens (paramTypes: ParamType[], values: TokenValue[]) {
    return Interface.encodeTokens(paramTypes, values);
  }
}

export default Interface;
