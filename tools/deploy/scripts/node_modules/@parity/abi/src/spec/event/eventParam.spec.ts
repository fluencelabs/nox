// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import EventParam from './eventParam';

describe('spec/event/EventParam', () => {
  describe('constructor', () => {
    it('sets the properties', () => {
      const param = new EventParam('foo', 'uint', true);

      expect(param.name).toEqual('foo');
      expect(param.kind.type).toEqual('uint');
      expect(param.indexed).toBe(true);
    });

    it('uses defaults for indexed', () => {
      expect(new EventParam('foo', 'uint').indexed).toBe(false);
    });
  });

  describe('toEventParams', () => {
    it('maps an array of params', () => {
      const params = EventParam.toEventParams([{ name: 'foo', type: 'uint' }]);

      expect(params.length).toEqual(1);
      expect(params[0].indexed).toBe(false);
      expect(params[0].name).toEqual('foo');
      expect(params[0].kind.type).toEqual('uint');
    });
  });
});
