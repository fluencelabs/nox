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

class PubSubBase {
  // Provider for websocket pubsub transport
  constructor (provider) {
    this._provider = provider;
  }

  addListener (module, eventName, callback, eventParams = []) {
    if (eventName) {
      const params = eventParams
        ? [eventName, eventParams]
        : [eventName];

      return this._provider.subscribe(module, callback, params);
    }

    return this._provider.subscribe(module, callback, eventParams);
  }

  removeListener (subscriptionIds) {
    return this._provider.unsubscribe(subscriptionIds);
  }

  unsubscribe (subscriptionIds) {
    return this.removeListener(subscriptionIds);
  }
}

module.exports = PubSubBase;
