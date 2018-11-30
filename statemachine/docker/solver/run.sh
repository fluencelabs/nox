#!/bin/bash -e
set -e

# param
# $1 tm_home
# $2 predefined_config_directory
# $3 state_machine_target_path

TM_HOME="$1"    # /tendermint
CONFIG_DIR="$2" # /solver
JAR="$3"        # /app/solver.jar

if [ "$#" -ne 3 ]; then
    echo "Illegal number of parameters: 3 required"
    exit 1
fi

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


cp -rf "$SOLVER_DIR/." "$TM_HOME/"
cp -rf "$CODE_DIR/." "/vmcode/"

# copy predefined Tendermint config
cp -f "$CONFIG_DIR/config.toml" "$TM_HOME/config/config.toml"

# extract cluster and info
node_info_file="$TM_HOME/config/node_info.json"
node_index=$(cat "$node_info_file" | jq -r .node_index)

persistent_peers=$(cat "$node_info_file" | jq -r ".cluster|.persistent_peers")
external_address=$(cat "$node_info_file" | jq -r ".cluster|.external_addrs|.[$node_index]")

cat "$node_info_file" | jq ".cluster|.genesis" > "$TM_HOME/config/genesis.json"

# set advertised address
sed -i -e "s#^external_address = \"\"\$#external_address = \"$external_address\"#" "$TM_HOME/config/config.toml"

# run Tendermint with disabled output
tendermint node "--home=$TM_HOME" "--moniker=$HOSTNAME" "--p2p.persistent_peers=$persistent_peers" > /dev/null 2> /dev/null &

# run State machine
java -Dconfig.file="$CONFIG_DIR/sm.config" -jar "$JAR"
