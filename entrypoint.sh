#! /usr/bin/env bahs

mkdir -p ${FLUENCE_BASE_DIR}
chown -r ${FLUENCE_UID}:${FLUENCE_GID} ${FLUENCE_BASE_DIR}

exec gosu fluence nox "$@"
