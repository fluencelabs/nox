#! /usr/bin/env bash

export FLUENCE_UID=${FLUENCE_UID:-1000}
export FLUENCE_BASE_DIR="${FLUENCE_BASE_DIR:-/.fluence}"
export FLUENCE_CONFIG="${FLUENCE_CONFIG:-$FLUENCE_BASE_DIR/v1/Config.toml}"

useradd --uid "$FLUENCE_UID" --gid 100 --no-create-home --shell /usr/sbin/nologin fluence >&2
mkdir -p ${FLUENCE_BASE_DIR}
chown -R ${FLUENCE_UID}:100 ${FLUENCE_BASE_DIR}

exec gosu fluence nox "$@"
