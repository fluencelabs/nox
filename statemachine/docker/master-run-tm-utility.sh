#!/bin/bash -e
# param
# $1 utility_command_location
# $2 tm_home

if [ "$#" -ne 2 ]; then
    echo "Illegal number of parameters: 2 required"
    exit 1
fi

docker run --user $(id -u):$(id -g) -v "$PWD/$1:/solver" -v "$2:/tendermint" fluencelabs/solver:latest
