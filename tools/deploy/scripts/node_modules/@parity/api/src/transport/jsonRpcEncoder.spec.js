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

const JsonRpcEncoder = require('./jsonRpcEncoder');

const encoder = new JsonRpcEncoder();

describe('transport/JsonRpcEncoder', () => {
  describe('encodeObject', () => {
    it('encodes the body correctly, incrementing id', () => {
      const id = encoder.id;
      const bdy = encoder.encodeObject('someMethod', ['param1', 'param2']);
      const enc = {
        id,
        jsonrpc: '2.0',
        method: 'someMethod',
        params: ['param1', 'param2']
      };

      expect(bdy).to.deep.equal(enc);
      expect(encoder.id - id).to.equal(1);
    });
  });

  describe('encode', () => {
    it('encodes the body correctly, incrementing id', () => {
      const id = encoder.id;
      const bdy = encoder.encode('someMethod', ['param1', 'param2']);
      const enc = `{"id":${id},"jsonrpc":"2.0","method":"someMethod","params":["param1","param2"]}`;

      expect(bdy).to.equal(enc);
      expect(encoder.id - id).to.equal(1);
    });
  });
});
