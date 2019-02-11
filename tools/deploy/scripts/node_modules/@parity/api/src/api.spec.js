// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

/* eslint-disable no-unused-expressions */

const sinon = require('sinon');

const util = require('./util');
const Api = require('./api');

describe('Api', () => {
  describe('constructor', () => {
    it('wraps a currentProvider when supplied', () => {
      const sendAsync = sinon.stub();
      const currentProvider = {
        sendAsync
      };
      const api = new Api(currentProvider);

      api.provider.send('method', [], () => {});
      expect(sendAsync).to.have.been.called;
    });
  });

  it('exposes util as static property', () => {
    expect(Api.util).to.equal(util);
  });
});
