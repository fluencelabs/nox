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

const nock = require('nock');
const MockWsServer = require('mock-socket').Server;

const TEST_HTTP_URL = 'http://localhost:6688';
const TEST_WS_URL = 'ws://localhost:8866';

function mockHttp (requests) {
  nock.cleanAll();
  let scope = nock(TEST_HTTP_URL);

  requests.forEach((request, index) => {
    scope = scope
      .post('/')
      .reply(request.code || 200, (uri, body) => {
        if (body.method !== request.method) {
          return {
            error: `Invalid method ${body.method}, expected ${request.method}`
          };
        }

        scope.body = scope.body || {};
        scope.body[request.method] = body;

        return request.reply;
      });
  });

  return scope;
}

function mockWs (requests) {
  let mockServer = new MockWsServer(TEST_WS_URL);
  const scope = { requests: 0, body: {}, server: mockServer };

  scope.isDone = () => scope.requests === requests.length;
  scope.stop = () => {
    if (mockServer) {
      mockServer.stop();
      mockServer = null;
    }
  };

  mockServer.on('message', (_body) => {
    const body = JSON.parse(_body);
    const request = requests[scope.requests];
    const reply = request.reply;
    const response = reply.error
      ? { id: body.id, error: { code: reply.error.code, message: reply.error.message } }
      : { id: body.id, result: reply };

    scope.body[request.method] = body;
    scope.requests++;

    mockServer.send(JSON.stringify(response));

    if (request.method.match('subscribe') && request.subscription) {
      mockServer.send(JSON.stringify(request.subscription));
    }
  });

  return scope;
}

module.exports = {
  TEST_HTTP_URL,
  TEST_WS_URL,
  mockHttp,
  mockWs
};
