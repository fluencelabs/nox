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

const PubsubBase = require('../pubsubBase');
const { outSignerRequest } = require('../../format/output');

class Signer extends PubsubBase {
  constructor (provider) {
    super(provider);

    this._api = {
      subscribe: 'signer_subscribePending',
      unsubscribe: 'signer_unsubscribePending',
      subscription: 'signer_pending'
    };
  }

  pendingRequests (callback) {
    return this.addListener(this._api, null, (error, data) => {
      error
        ? callback(error)
        : callback(null, data.map(outSignerRequest));
    });
  }

  requestsToConfirm (callback) {
    return this.pendingRequests(callback);
  }
}

module.exports = Signer;
