#!/bin/bash

# Uses to start Fluence node with Swarm and Ethereum node on Kovan chain.

set -e

USAGE="Usage: ./kovan-compose.sh <environment> <external-ip> <owner-address> <private-key> <start-port:end-port>"

if [ ! $1 = '--help' -a ! $1 = '-h' ]; then

    if [ $# -eq 5 ]; then

        # read `deploy.config` file as a source without first line
        eval $(cat ../deploy.config | (read; cat))

        export PROD_DEPLOY='true'
        export CHAIN='kovan'
        export NAME='fluence-node-1'

        # get dynamic variable from `deploy.config`
        export CONTRACT_ADDRESS=$(eval echo "\$$1")

        export HOST_IP="$2"
        export OWNER_ADDRESS="$3"
        export PRIVATE_KEY="$4"
        export PORTS="$5"
        ./compose.sh
    else
        echo "Error: Not enough arguments."
        echo $USAGE
        exit 125
    fi
else
    echo $USAGE
fi
