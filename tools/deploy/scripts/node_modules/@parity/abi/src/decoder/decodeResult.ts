// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import Token from '../token';

class DecodeResult {
  _token: Token;
  _newOffset: number;

  constructor (token: Token, newOffset: number) {
    this._token = token;
    this._newOffset = newOffset;
  }

  get token () {
    return this._token;
  }

  get newOffset () {
    return this._newOffset;
  }
}

export default DecodeResult;
