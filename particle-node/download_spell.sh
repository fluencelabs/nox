#!/usr/bin/env bash
set -o pipefail -o nounset -o errexit

# get script directory
CUR_DIR="$(dirname "$0")"
SPELL_DIR="${CUR_DIR}/spell"
TAR="spell.tar.gz"

echo "*** downloading $TAR ***"
URL="https://github.com/fluencelabs/spell/releases/latest/download/spell.tar.gz"
curl --fail -L "$URL" -o "$TAR"

echo "*** extracting $TAR ***"
mkdir -p "$SPELL_DIR"
tar -C "$SPELL_DIR" -xf "$TAR"

rm "$TAR"
echo "*** done ***"

