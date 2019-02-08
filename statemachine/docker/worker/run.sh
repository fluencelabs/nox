# Copyright 2018 Fluence Labs Limited
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

#!/bin/bash -e

set -e

JAR="$1"

if [ -z "$CODE_DIR" ]; then
  cat >&2 <<EOF
error: \`-e "CODE_DIR=path_to_code"\` was not specified.
EOF
  exit 1
fi

if [ -z "$WORKER_DIR" ]; then
  cat >&2 <<EOF
error: \`-e "WORKER_DIR=path_to_worker_config"\` was not specified.
EOF
  exit 1
fi

cp -rf "$WORKER_DIR/." "/tendermint/"
cp -rf "$CODE_DIR/." "/vmcode/"

# run Tendermint with disabled output
tendermint node "--home=/tendermint" &

TENDERMINT_PID=$!

# run State machine
java -jar "$JAR" &

STATEMACHINE_PID=$!

# TODO: remove all that trapping logic after tendermint and statemachine are separated into independent containers
# Set trap for docker stop
trap 'kill -s INT %1; kill -s INT %2; echo "KILLING %1 %2 with SIGINT"' INT
# Set trap for Ctrl-C, just in case
trap 'kill -s TERM %1; kill -s TERM %2; echo "KILLING %1 %2 with SIGTERM"' TERM

# first `wait` will exit after receiving a SIG
wait $TENDERMINT_PID
# waiting second time to wait until tendermint is really exited
wait $TENDERMINT_PID

# first `wait` will exit after receiving a SIG (not sure if it's needed, but no harm here for sure)
wait $STATEMACHINE_PID
# waiting second time to wait until statemachine (java) is really exited
wait $STATEMACHINE_PID
