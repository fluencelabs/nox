#!/bin/bash -e
# param
# $1 tm_home
# $2 predefined_config_directory
# $3 state_machine_target_path

if [ "$#" -ne 3 ]; then
    echo "Illegal number of parameters: 3 required"
    exit 1
fi

# uncomment line below if attaching to container is needed before node running
#sleep 8

# copy predefined Tendermint config
cp -f "$2/config.toml" "$1/config/config.toml"

# extract cluster and info
node_info_file="$1/config/node_info.json"
node_index=$(cat "$node_info_file" | jq -r .node_index)

persistent_peers=$(cat "$node_info_file" | jq -r ".cluster|.persistent_peers")
external_address=$(cat "$node_info_file" | jq -r ".cluster|.external_addrs|.[$node_index]")

cat "$node_info_file" | jq ".cluster|.genesis" > "$1/config/genesis.json"

# set advertised address
sed -i -e "s#^external_address = \"\"\$#external_address = \"$external_address\"#" "$1/config/config.toml"

# run Tendermint with disabled output
tendermint node "--home=$1" "--moniker=$HOSTNAME" "--p2p.persistent_peers=$persistent_peers" > /dev/null 2> /dev/null &

# run State machine
java -Dconfig.file="$2/sm.config" -jar "$3"
