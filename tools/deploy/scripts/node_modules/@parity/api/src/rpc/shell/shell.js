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

class Shell {
  constructor (provider) {
    this._provider = provider;
  }

  getApps (retrieveAll = false) {
    return this._provider
      .send('shell_getApps', retrieveAll);
  }

  getFilteredMethods () {
    return this._provider
      .send('shell_getFilteredMethods');
  }

  getMethodGroups () {
    return this._provider
      .send('shell_getMethodGroups');
  }

  getMethodPermissions () {
    return this._provider
      .send('shell_getMethodPermissions');
  }

  loadApp (appId, params = null) {
    return this._provider
      .send('shell_loadApp', appId, params);
  }

  setMethodPermissions (permissions) {
    return this._provider
      .send('shell_setMethodPermissions', permissions);
  }

  setAppVisibility (appId, state) {
    return this._provider
      .send('shell_setAppVisibility', appId, state);
  }

  setAppPinned (appId, state) {
    return this._provider
      .send('shell_setAppPinned', appId, state);
  }
}

module.exports = Shell;
