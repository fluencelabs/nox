# generates json with all arguments required for registration a node
function generate_json()
{
    # printing the command in the last line to parse it from a control script
    DATA=$(cat <<EOF
{
    "node_ip": "$EXTERNAL_IP",
    "ethereum_address": "$ETHEREUM_ADDRESS",
    "tendermint_key": "$TENDERMINT_KEY",
    "tendermint_node_id": "$TENDERMINT_NODE_ID",
    "contract_address": "$CONTRACT_ADDRESS",
    "account": "$OWNER_ADDRESS",
    "api_port": $API_PORT,
    "capacity": $CAPACITY
}
EOF
)
    JSON=$(echo $DATA | paste -sd "\0" -)
    echo "$JSON"
}

# parses tendermint node id and key from logs
function parse_tendermint_params()
{
    local __TENDERMINT_KEY=$1
    local __TENDERMINT_NODE_ID=$2

    # get tendermint key from node logs
    # todo get this from `status` API by CLI
    while [ -z "$TENDERMINT_KEY" -o -z "$TENDERMINT_NODE_ID" ]; do
        # check if docker container isn't in `exited` status
        local DOCKER_STATUS=$(docker ps -a --filter "name=fluence-node-1" --format '{{.Status}}' | grep -o Exited)
        if [ -n "$DOCKER_STATUS" ]; then
            echo -e >&2 "\e[91m'fluence-node-'1 container cannot be run\e[0m"
            exit 127
        fi

        TENDERMINT_KEY=$(docker logs fluence-node-1 2>&1 | awk 'match($0, /PubKey: /) { print substr($0, RSTART + RLENGTH) }')
        TENDERMINT_NODE_ID=$(docker logs fluence-node-1 2>&1 | awk 'match($0, /Node ID: /) { print substr($0, RSTART + RLENGTH) }')
        sleep 3
    done

    eval $__TENDERMINT_KEY="'$TENDERMINT_KEY'"
    eval $__TENDERMINT_NODE_ID="'$TENDERMINT_NODE_ID'"
}
