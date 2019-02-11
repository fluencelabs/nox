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

require('mock-local-storage');
require('isomorphic-fetch');

const es6Promise = require('es6-promise');

es6Promise.polyfill();

const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');

require('sinon-as-promised');

const sinonChai = require('sinon-chai');
const { JSDOM } = require('jsdom');
const { WebSocket } = require('mock-socket');

chai.use(chaiAsPromised);
chai.use(sinonChai);

const jsdom = new JSDOM('<!doctype html><html><body></body></html>');

global.expect = chai.expect;
global.WebSocket = WebSocket;

global.document = jsdom.window.document;
global.window = jsdom.window;
global.window.localStorage = global.localStorage;
global.navigator = global.window.navigator;
global.location = global.window.location;

module.exports = {};
