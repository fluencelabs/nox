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

const Contract = require('./contract');
const Providers = require('./provider');
const Transports = require('./transport');

const { Db, Eth, Parity, Net, Personal, Private, Shell, Shh, Signer, Trace, Web3 } = require('./rpc');
const Subscriptions = require('./subscriptions');
const Pubsub = require('./pubsub');
const util = require('./util');
const { isFunction } = require('./util/types');

class Api extends EventEmitter {
  constructor (provider, allowSubscriptions = true, middlewareClass) {
    super();

    if (!provider) {
      throw new Error('Provider needs to be supplied to Api instance');
    }

    if (isFunction(provider.sendAsync)) {
      provider = new Providers.Current(provider);
    } else if (!isFunction(provider.send)) {
      console.warn(
        new Error('deprecated: Api needs provider with send() function, old-style Transport found instead'),
        provider
      );
    }

    this._provider = new Providers.PromiseProvider(provider);
    this._provider.on('connected', () => this.emit('connected'));
    this._provider.on('connecting', () => this.emit('connecting'));
    this._provider.on('disconnected', () => this.emit('disconnected'));

    this._db = new Db(this._provider);
    this._eth = new Eth(this._provider);
    this._net = new Net(this._provider);
    this._parity = new Parity(this._provider);
    this._personal = new Personal(this._provider);
    this._private = new Private(this._provider);
    this._shell = new Shell(this._provider);
    this._shh = new Shh(this._provider);
    this._signer = new Signer(this._provider);
    this._trace = new Trace(this._provider);
    this._web3 = new Web3(this._provider);

    // FIXME: Remove, convert to shell
    if (middlewareClass) {
      const middleware = this.parity
        .nodeKind()
        .then(nodeKind => {
          if (nodeKind.availability === 'public') {
            return middlewareClass;
          }

          return null;
        })
        .catch(() => null);

      provider.addMiddleware(middleware);
    }

    if (provider && isFunction(provider.subscribe)) {
      this._pubsub = new Pubsub(provider);
    }

    if (allowSubscriptions) {
      this._subscriptions = new Subscriptions(this);
    }
  }

  get isConnected () {
    const isConnected = this.provider.isConnected;

    return isConnected || typeof isConnected === 'undefined';
  }

  get isPubSub () {
    return !!this._pubsub;
  }

  get pubsub () {
    if (!this.isPubSub) {
      throw Error('Pubsub is only available with a subscribing-supported transport injected!');
    }

    return this._pubsub;
  }

  get db () {
    return this._db;
  }

  get eth () {
    return this._eth;
  }

  get parity () {
    return this._parity;
  }

  get net () {
    return this._net;
  }

  get personal () {
    return this._personal;
  }

  get private () {
    return this._private;
  }

  get provider () {
    return this._provider.provider;
  }

  get shell () {
    return this._shell;
  }

  get shh () {
    return this._shh;
  }

  get signer () {
    return this._signer;
  }

  get trace () {
    return this._trace;
  }

  get transport () {
    return this.provider;
  }

  get web3 () {
    return this._web3;
  }

  get util () {
    return util;
  }

  newContract (abi, address) {
    return new Contract(this, abi).at(address);
  }

  subscribe (subscriptionName, callback) {
    if (!this._subscriptions) {
      return Promise.resolve(1);
    }

    return this._subscriptions.subscribe(subscriptionName, callback);
  }

  unsubscribe (subscriptionId) {
    if (!this._subscriptions) {
      return Promise.resolve(true);
    }

    return this._subscriptions.unsubscribe(subscriptionId);
  }

  pollMethod (method, input, validate) {
    const [_group, endpoint] = method.split('_');
    const group = `_${_group}`;

    return new Promise((resolve, reject) => {
      const timeout = () => {
        this[group][endpoint](input)
          .then(result => {
            if (validate ? validate(result) : result) {
              resolve(result);
            } else {
              setTimeout(timeout, 500);
            }
          })
          .catch(error => {
            // Don't print if the request is rejected: that's ok
            if (error.type !== 'REQUEST_REJECTED') {
              console.error('pollMethod', error);
            }

            reject(error);
          });
      };

      timeout();
    });
  }
}

Api.util = util;

Api.Provider = {
  Current: Providers.Current,
  Ipc: Providers.Ipc,
  Http: Providers.Http,
  PostMessage: Providers.PostMessage,
  SendAsync: Providers.SendAsync,
  Ws: Providers.Ws,
  WsSecure: Providers.WsSecure
};

// NOTE: kept for backwards compatibility
Api.Transport = {
  Http: Transports.Http,
  Ws: Transports.Ws,
  WsSecure: Transports.WsSecure
};

module.exports = Api;
