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
java -jar "$JAR"
