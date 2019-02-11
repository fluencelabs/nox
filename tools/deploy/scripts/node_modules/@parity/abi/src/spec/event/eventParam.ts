// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { AbiInput, TokenTypeEnum } from '../../types';
import Param from '../param';
import ParamType from '../paramType';
import { toParamType } from '../paramType/format';

class EventParam {
  private _indexed: boolean;
  private _kind: ParamType;
  private _name: string | undefined;

  static toEventParams (params: (Param | AbiInput)[]) {
    return params.map(
      param =>
        new EventParam(
          param.name,
          (param as Param).kind
            ? (param as Param).kind.type
            : ((param as AbiInput).type as TokenTypeEnum),
          (param as AbiInput).indexed
        )
    );
  }

  constructor (name: string | undefined, type: TokenTypeEnum, indexed = false) {
    this._name = name;
    this._indexed = indexed;
    this._kind = toParamType(type, indexed);
  }

  get name () {
    return this._name;
  }

  get kind () {
    return this._kind;
  }

  get indexed () {
    return this._indexed;
  }
}

export default EventParam;
