#!/bin/bash -e
# param
# $1 long_term_key_location

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters: 1 required"
    exit 1
fi

node_id=$(./master-run-tm-utility.sh tm-show-node-id "$1")
validator=$(./master-run-tm-utility.sh tm-show-validator "$1")

# create a JSON document with public keys
echo "{\"node_id\":\"$node_id\",\"validator\":$validator}"
