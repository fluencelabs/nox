#!/bin/bash -e
set -e

JAR="$1"

if [ -z "$CODE_DIR" ]; then
  cat >&2 <<EOF
error: \`-e "CODE_DIR=path_to_code"\` was not specified.
EOF
  exit 1
fi

if [ -z "$SOLVER_DIR" ]; then
  cat >&2 <<EOF
error: \`-e "SOLVER_DIR=path_to_solver_config"\` was not specified.
EOF
  exit 1
fi

cp -rf "$SOLVER_DIR/." "/tendermint/"
cp -rf "$CODE_DIR/." "/vmcode/"

# run Tendermint with disabled output
tendermint node "--home=/tendermint" &

# run State machine
java -Dconfig.file="/solver/sm.config" -jar "$JAR"
