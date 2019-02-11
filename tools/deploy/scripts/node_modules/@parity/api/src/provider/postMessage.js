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

class PostMessage extends EventEmitter {
  constructor (appId, destination, source) {
    super();

    this._appId = appId;
    this._destination = destination || window.parent;

    this.id = 0;
    this._connected = false;
    this._messages = {};
    this._queued = [];

    this._receiveMessage = this._receiveMessage.bind(this);
    this._send = this._send.bind(this);
    this.send = this.send.bind(this);
    this.subscribe = this.subscribe.bind(this);
    this.unsubscribe = this.unsubscribe.bind(this);

    (source || window).addEventListener('message', this._receiveMessage, false);
  }

  get isConnected () {
    return this._connected;
  }

  get isParity () {
    return true;
  }

  get queuedCount () {
    return this._queued.length;
  }

  setToken (token) {
    if (token) {
      this._connected = true;
      this._token = token;
      this.emit('connected');
      this._sendQueued();
    }
  }

  addMiddleware () {
  }

  _constructMessage (id, data) {
    return Object.assign({}, data, {
      id,
      to: 'shell',
      from: this._appId,
      token: this._token
    });
  }

  _send (message) {
    if (!this._token) {
      this._queued.push(message);

      return;
    }

    const id = ++this.id;
    const postMessage = this._constructMessage(id, message.data);

    this._messages[id] = Object.assign({}, postMessage, message.options);
    this._destination.postMessage(postMessage, '*');
  }

  send (method, params, callback) {
    this._send({
      data: {
        method,
        params
      },
      options: {
        callback
      }
    });
  }

  _sendQueued () {
    if (!this._token) {
      return;
    }

    this._queued.forEach(this._send);
    this._queued = [];
  }

  subscribe (api, callback, params) {
    // console.log('paritySubscribe', JSON.stringify(params), api, callback);
    return new Promise((resolve, reject) => {
      this._send({
        data: {
          api,
          params
        },
        options: {
          callback,
          resolve,
          reject,
          subscription: true,
          initial: true
        }
      });
    });
  }

  // FIXME: Should return callback, not promise
  unsubscribe (subId) {
    return new Promise((resolve, reject) => {
      this._send({
        data: {
          subId
        },
        options: {
          callback: (error, result) => {
            error
              ? reject(error)
              : resolve(result);
          }
        }
      });
    });
  }

  unsubscribeAll () {
    return this.unsubscribe('*');
  }

  _receiveMessage ({ data: { id, error, from, to, token, result }, origin, source }) {
    const isTokenValid = token
      ? token === this._token
      : true;

    if (from !== 'shell' || to !== this._appId || !isTokenValid) {
      return;
    }

    if (this._messages[id].subscription) {
      // console.log('subscription', result, 'initial?', this._messages[id].initial);
      this._messages[id].initial
        ? this._messages[id].resolve(result)
        : this._messages[id].callback(error && new Error(error), result);
      this._messages[id].initial = false;
    } else {
      this._messages[id].callback(error && new Error(error), result);
      this._messages[id] = null;
    }
  }
}

module.exports = PostMessage;
