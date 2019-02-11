// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import DecodedLog from './decodedLog';
import DecodedLogParam from './decodedLogParam';
import ParamType from '../paramType';
import Token from '../../token';

const decodedLogParam = new DecodedLogParam(
  'someparams',
  new ParamType('address'),
  new Token('address')
);
const log = new DecodedLog([decodedLogParam], 'someAddress');

describe('spec/event/DecodedLog', () => {
  describe('constructor', () => {
    it('sets internal state', () => {
      expect(log.params).toEqual([decodedLogParam]);
      expect(log.address).toEqual('someAddress');
    });
  });
});
