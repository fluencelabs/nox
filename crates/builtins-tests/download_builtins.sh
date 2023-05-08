#!/usr/bin/env bash
set -o pipefail -o nounset -o errexit

# get script directory
BUILTINS_DIR="$(dirname "$0")"
SERVICES_DIR="${BUILTINS_DIR}/../particle-node-tests/tests/builtins/services"
TAR="registry.tar.gz"

echo "*** downloading $TAR ***"
URL="https://github.com/fluencelabs/registry/releases/download/v0.8.5/registry.tar.gz"
curl --fail -L "$URL" -o "$TAR"

echo "*** extracting $TAR ***"
mkdir -p "$SERVICES_DIR"
tar -C "$SERVICES_DIR" -xf "$TAR"

rm "$TAR"

echo "*** done ***"
