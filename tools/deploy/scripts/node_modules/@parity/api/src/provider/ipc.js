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

// https://github.com/electron/electron/issues/2288
const IS_ELECTRON = !!(typeof window !== 'undefined' && window && window.process && window.process.type);

let ipcRenderer;

if (IS_ELECTRON) {
  ipcRenderer = window.require('electron').ipcRenderer;
}

const METHOD_REQUEST_TOKEN = 'shell_requestNewToken';

class Ipc extends EventEmitter {
  constructor (appId) {
    super();
    this._appId = appId;

    this.id = 0;
    this._messages = {};
    this._queued = [];

    if (!IS_ELECTRON) {
      throw new Error('IpcProvider must be used in Electron environment.');
    }

    ipcRenderer.on('PARITY_SHELL_IPC_CHANNEL', this.receiveMessage.bind(this));
  }

  _constructMessage (id, data) {
    return Object.assign({}, data, {
      id,
      to: 'shell',
      from: this._appId,
      token: this._token
    });
  }

  receiveMessage (_, { id, error, from, to, token, result }) {
    const isTokenValid = token ? token === this._token : true;

    if (from !== 'shell' || to !== this._appId || !isTokenValid) {
      return;
    }

    if (this._messages[id].subscription) {
      this._messages[id].initial
        ? this._messages[id].resolve(result)
        : this._messages[id].callback(error && new Error(error), result);
      this._messages[id].initial = false;
    } else {
      this._messages[id].callback(error && new Error(error), result);
      this._messages[id] = null;
    }
  }

  requestNewToken () {
    return new Promise((resolve, reject) => {
      // Webview is ready when receivin the ping
      ipcRenderer.once('ping', () => {
        this.send(METHOD_REQUEST_TOKEN, [], (error, token) => {
          if (error) {
            reject(error);
          } else {
            this.setToken(token);
            resolve(token);
          }
        });
      });
    });
  }

  _send (message) {
    if (!this._token && message.data.method !== METHOD_REQUEST_TOKEN) {
      this._queued.push(message);

      return;
    }

    const id = ++this.id;
    const postMessage = this._constructMessage(id, message.data);

    this._messages[id] = Object.assign({}, postMessage, message.options);

    ipcRenderer.sendToHost('parity', { data: postMessage });
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

    this._queued.forEach(this._send.bind(this));
    this._queued = [];
  }

  setToken (token) {
    if (token) {
      this._connected = true;
      this._token = token;
      this.emit('connected');
      this._sendQueued();
    }
  }

  subscribe (api, callback, params) {
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
            error ? reject(error) : resolve(result);
          }
        }
      });
    });
  }

  unsubscribeAll () {
    return this.unsubscribe('*');
  }
}

module.exports = Ipc;
