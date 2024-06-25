#! /usr/bin/env bash

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

export FLUENCE_UID=${FLUENCE_UID:-1000}
export FLUENCE_BASE_DIR="${FLUENCE_BASE_DIR:-/.fluence}"
export FLUENCE_CONFIG="${FLUENCE_CONFIG:-$FLUENCE_BASE_DIR/v1/Config.toml}"

useradd --uid "$FLUENCE_UID" --gid 100 --no-create-home --shell /usr/sbin/nologin fluence >&2
mkdir -p ${FLUENCE_BASE_DIR}
chown -R ${FLUENCE_UID}:100 ${FLUENCE_BASE_DIR}

exec gosu fluence nox "$@"
