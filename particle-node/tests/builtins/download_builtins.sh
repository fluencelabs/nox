#!/usr/bin/env bash
set -o pipefail -o nounset -o errexit

# get script directory
BUILTINS_DIR="$(dirname "$0")"
SERVICES_DIR="${BUILTINS_DIR}/services"
TAR="aqua-dht.tar.gz"

echo "*** downloading $TAR ***"
URL="https://github.com/fluencelabs/aqua-dht/releases/latest/download/aqua-dht.tar.gz"
curl --fail -L "$URL" -o $TAR

echo "*** extracting $TAR ***"
tar -C $SERVICES_DIR -xf $TAR

rm $TAR

echo "*** done ***"
