// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import DecodedLogParam from './decodedLogParam';
import ParamType from '../paramType';
import Token from '../../token';

describe('spec/event/DecodedLogParam', () => {
  describe('constructor', () => {
    const pt = new ParamType('bool');
    const tk = new Token('bool');

    it('disallows kind not instanceof ParamType', () => {
      expect(
        () => new DecodedLogParam('test', 'param' as any, undefined as any)
      ).toThrow(/ParamType/);
    });

    it('disallows token not instanceof Token', () => {
      expect(() => new DecodedLogParam('test', pt, 'token' as any)).toThrow(
        /Token/
      );
    });

    it('stores all parameters received', () => {
      const log = new DecodedLogParam('test', pt, tk);

      expect(log.name).toEqual('test');
      expect(log.kind).toEqual(pt);
      expect(log.token).toEqual(tk);
    });
  });
});
