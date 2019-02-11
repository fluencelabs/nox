// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

class BytesTaken {
  private _bytes: number[];
  private _newOffset: number;

  constructor (bytes: number[], newOffset: number) {
    this._bytes = bytes;
    this._newOffset = newOffset;
  }

  get bytes () {
    return this._bytes;
  }

  get newOffset () {
    return this._newOffset;
  }
}

export default BytesTaken;
