// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { AbiItem } from '../types';
import Encoder from '../encoder/encoder';
import Param from './param';
import Token from '../token';

class Constructor {
  private _inputs: Param[];

  constructor (abi: AbiItem) {
    this._inputs = Param.toParams(abi.inputs || []);
  }

  get inputs () {
    return this._inputs;
  }

  inputParamTypes () {
    return this._inputs.map(input => input.kind);
  }

  encodeCall (tokens: Token[]) {
    return Encoder.encode(tokens);
  }
}

export default Constructor;
