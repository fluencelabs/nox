#!/bin/sh

#
# Nox Fluence Peer
#
# Copyright (C) 2024 Fluence DAO
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as
# published by the Free Software Foundation version 3 of the
# License.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#

# This script builds all subprojects and puts all created Wasm modules in one dir
marine --version
marine build --release

mkdir -p artifacts
rm -f artifacts/*
cp target/wasm32-wasi/release/file_share.wasm artifacts/
