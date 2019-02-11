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

const PostMessage = require('./postMessage');

const APP_ID = 'xyz';

let destination;
let provider;
let source;

function createDestination () {
  destination = {
    postMessage: sinon.stub()
  };

  return destination;
}

function createSource () {
  source = {
    addEventListener: sinon.stub()
  };

  return source;
}

function createProvider () {
  provider = new PostMessage(APP_ID, createDestination(), createSource());

  return provider;
}

describe('provider/PostMessage', () => {
  beforeEach(() => {
    createProvider();
  });

  describe('constructor', () => {
    it('adds an event listener for message', () => {
      expect(source.addEventListener).to.have.been.calledWith('message', provider._receiveMessage);
    });
  });

  describe('getters', () => {
    describe('isParity', () => {
      it('returns true', () => {
        expect(provider.isParity).to.be.true;
      });
    });

    describe('isConnected', () => {
      it('returns the internal connected status', () => {
        provider._connected = 'connected';

        expect(provider.isConnected).to.equal('connected');
      });
    });
  });

  describe('setToken', () => {
    beforeEach(() => {
      sinon.spy(provider, '_sendQueued');
    });

    afterEach(() => {
      provider._sendQueued.restore();
    });

    it('sets the connected status', () => {
      expect(provider.isConnected).to.be.false;

      provider.setToken('123');

      expect(provider.isConnected).to.be.true;
    });

    it('sends all queued messages', () => {
      expect(provider._sendQueued).not.to.have.beenCalled;

      provider.setToken('123');

      expect(provider._sendQueued).to.have.been.called;
    });

    it('emits a connected message', (done) => {
      provider.on('connected', done);
      provider.setToken('123');
    });
  });

  describe('send', () => {
    it('queues messages before token is available', () => {
      expect(provider.queuedCount).to.equal(0);

      provider.send('method', 'params', () => {});

      expect(destination.postMessage).not.to.have.been.called;
      expect(provider.queuedCount).to.equal(1);
    });

    it('sends queued messages as token is available', () => {
      expect(provider.queuedCount).to.equal(0);

      provider.send('method', 'params', () => {});
      provider.setToken('123');

      expect(destination.postMessage).to.have.been.calledWith(
        provider._constructMessage(1, {
          method: 'method',
          params: 'params'
        }), '*'
      );
      expect(provider.queuedCount).to.equal(0);
    });
  });
});
