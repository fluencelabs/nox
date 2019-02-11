// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import Param from './param';

describe('spec/Param', () => {
  describe('constructor', () => {
    const param = new Param('foo', 'uint');

    it('sets the properties', () => {
      expect(param.name).toEqual('foo');
      expect(param.kind.type).toEqual('uint');
    });
  });

  describe('toParams', () => {
    it('maps an array of params', () => {
      const params = Param.toParams([{ name: 'foo', type: 'uint' }]);

      expect(params.length).toEqual(1);
      expect(params[0].name).toEqual('foo');
      expect(params[0].kind.type).toEqual('uint');
    });

    it('converts only if needed', () => {
      const _params = Param.toParams([{ name: 'foo', type: 'uint' }]);
      const params = Param.toParams(_params);

      expect(params.length).toEqual(1);
      expect(params[0].name).toEqual('foo');
      expect(params[0].kind.type).toEqual('uint');
    });
  });
});
