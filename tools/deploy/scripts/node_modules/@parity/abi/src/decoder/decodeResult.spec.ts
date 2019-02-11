// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import DecodeResult from './decodeResult';
import Token from '../token';

describe('decoder/DecodeResult', () => {
  describe('constructor', () => {
    it('sets the token of the object', () => {
      const token = new Token('bool', 1);
      expect(new DecodeResult(token, 2).token).toEqual(token);
    });

    it('sets the newOffset of the object', () => {
      expect(new DecodeResult(new Token('bool', 1), 4).newOffset).toEqual(4);
    });
  });
});
