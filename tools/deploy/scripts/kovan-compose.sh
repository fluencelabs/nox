#!/bin/bash

# Uses to start Fluence node with Swarm and Ethereum node on Kovan chain.

set -e

USAGE="Usage: ./kovan-compose.sh <external-ip> <owner-address> <private-key> <start-port:end-port>"

if [ ! $1 = '--help' -a ! $1 = '-h' ]; then

    if [ $# -eq 4 ]; then

        if [ ! -f contract.txt ]; then
            echo "File contract.txt not found!"
            exit 125
        fi

        export PROD_DEPLOY='true'
        export CHAIN='kovan'
        export NAME='fluence-node-1'
        export CONTRACT_ADDRESS=$(cat contract.txt)
        export HOST_IP="$1"
        export OWNER_ADDRESS="$2"
        export PRIVATE_KEY="$3"
        export PORTS="$4"
        ./compose.sh
    else
        echo "Error: Not enough arguments."
        echo $USAGE
        exit 125
    fi
else
    echo $USAGE
fi
