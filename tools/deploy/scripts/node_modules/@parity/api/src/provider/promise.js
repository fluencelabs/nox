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

class PromiseProvider extends EventEmitter {
  constructor (provider) {
    super();

    this.provider = provider;
    this.provider.on('connected', () => this.emit('connected'));
    this.provider.on('connecting', () => this.emit('connecting'));
    this.provider.on('disconnected', () => this.emit('disconnected'));

    this.send = this.send.bind(this);
  }

  get isConnected () {
    return this.provider.isConnected;
  }

  get isParity () {
    return !!this.provider.isParity;
  }

  send (method, ...params) {
    if (!this.provider.send) {
      // old-style transport interface for backwards compatibility
      return this.provider.execute(method, params);
    }

    return new Promise((resolve, reject) => {
      this.provider.send(method, params, (error, result) => {
        if (error) {
          reject(error);
        } else {
          resolve(result);
        }
      });
    });
  }
}

module.exports = PromiseProvider;
