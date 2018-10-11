#!/bin/bash
# param
# $1 cluster_name
# $2, $3, $4, $5 nodes' keys

# iterate through given node public key JSONs, combine genesis info and persistent peers
for ((i = 2; i <= $#; i++)); do
    node_name="node"$(($i-2))
    node_addr=$1"_"$node_name":26656"

    validator_key=$(echo ${!i} | jq .validator)
    node_id=$(echo ${!i} | jq -r .node_id)

    validators=$validators"{\"pub_key\":$validator_key,\"power\":\"1\",\"name\":\"$node_name\"},"
    persistent_peers=$persistent_peers$node_id@$node_addr","
    external_addrs=$external_addrs"\""$node_addr"\","
done

# remove trailing commas
validators=${validators%,}
persistent_peers=${persistent_peers%,}
external_addrs=${external_addrs%,}

# create genesis JSON according to Tendermint format
genesis_time=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
genesis="{\"genesis_time\":\"$genesis_time\",\"chain_id\":\"$1\",\"app_hash\":\"\",\"validators\":[$validators]}"

# combine JSON with cluster genesis, persistent peers, and external addresses
echo "{\"genesis\":$genesis,\"persistent_peers\":\"$persistent_peers\",\"external_addrs\":[$external_addrs]}"
