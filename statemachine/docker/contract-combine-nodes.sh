#!/bin/bash -e
# param
# $1 cluster_name
# $2 host_base_rpc_port
# $3, $4, $5, $6 nodes' keys

if [ "$#" -lt 3 ]; then
    echo "Illegal number of parameters: at least 3 required"
    exit 1
fi

# detecting how host seen from container
# docker for Mac/Win maps 'host.docker.internal' to the host
if [ "$(uname)" == "Darwin" ]; then
    host_docker_internal="host.docker.internal"
else
    host_docker_internal=$(ifconfig docker0 | grep 'inet ' | awk '{print $2}')
fi

# iterate through given node public key JSONs, combine genesis info and persistent peers
for ((i = 3; i <= $#; i++)); do
    node_index=$(($i-3))
    node_name=node$node_index
    node_addr=$host_docker_internal:$(($node_index * 100 + $2 - 1))

    ith_arg=${!i}
    validator_key=$(cut -d';' -f1 <<<"$ith_arg")
    node_id=$(cut -d';' -f2 <<<"$ith_arg")

    current_validator=$(cat <<EOF
        {
            "pub_key": $validator_key,
            "power": "1",
            "name": "$node_name"
        }
EOF
)

    validators=$validators$current_validator,
    persistent_peers=$persistent_peers$node_id@$node_addr,
    external_addrs=$external_addrs"\""$node_addr"\","
done

# remove trailing commas
validators=${validators%,}
persistent_peers=${persistent_peers%,}
external_addrs=${external_addrs%,}

# create genesis JSON according to Tendermint format
genesis_time=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
genesis=$(cat <<EOF
    {
        "genesis_time": "$genesis_time",
        "chain_id": "$1",
        "app_hash": "",
        "validators": [$validators]
    }
EOF
)

result_doc=$(cat <<EOF
    {
        "genesis": $genesis,
        "persistent_peers": "$persistent_peers",
        "external_addrs": [$external_addrs]
    }
EOF
)

# combine JSON with cluster genesis, persistent peers, and external addresses
echo $result_doc
