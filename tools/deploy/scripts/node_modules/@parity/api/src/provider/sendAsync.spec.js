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

const SendAsync = require('./sendAsync');

function initProvider () {
  return new SendAsync({
    send: (method, params, callback) => {
      callback(null, { isStubbed: true, method, params });
    }
  });
}

describe('provider/SendAsync', () => {
  describe('send', () => {
    it('throws an expection', () => {
      expect(() => initProvider().send()).to.throw;
    });
  });

  describe('sendAsync', () => {
    it('calls into the supplied provider', (done) => {
      initProvider().sendAsync({ method: 'method', params: 'params' }, (error, data) => { // eslint-disable-line
        expect(data).to.deep.equal({
          result: {
            isStubbed: true,
            method: 'method',
            params: 'params'
          }
        });
        done();
      });
    });
  });
});
