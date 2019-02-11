// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { isInstanceOf } from '../../util/types';
import ParamType from '../paramType';
import Token from '../../token';

class DecodedLogParam {
  private _kind: ParamType;
  private _name: string | undefined;
  private _token: Token;

  constructor (name: string | undefined, kind: ParamType, token: Token) {
    if (!isInstanceOf(kind, ParamType)) {
      throw new Error('kind not instanceof ParamType');
    } else if (!isInstanceOf(token, Token)) {
      throw new Error('token not instanceof Token');
    }

    this._name = name;
    this._kind = kind;
    this._token = token;
  }

  get name () {
    return this._name;
  }

  get kind () {
    return this._kind;
  }

  get token () {
    return this._token;
  }
}

export default DecodedLogParam;
