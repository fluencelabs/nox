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

const EventEmitter = require('eventemitter3');

class JsonRpcEncoder extends EventEmitter {
  constructor () {
    super();

    this._id = 1;
  }

  encodeObject (method, params) {
    return {
      id: this._id++,
      jsonrpc: '2.0',
      method: method,
      params: params
    };
  }

  encode (method, params) {
    return JSON.stringify(this.encodeObject(method, params));
  }

  get id () {
    return this._id;
  }
}

module.exports = JsonRpcEncoder;
