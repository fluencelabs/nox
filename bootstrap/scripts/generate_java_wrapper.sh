#!/bin/bash -e

command -v web3j >/dev/null 2>&1 || { 
    echo >&2 "web3j is not installed. See https://web3j.readthedocs.io/en/latest/command_line.html" 
    exit 1 
}

WEB3J=$(command -v web3j)
BOOTSTRAP_DIR=$(pwd)
CONTRACTS_DIR=$BOOTSTRAP_DIR/contracts
COMPILED_DIR=$CONTRACTS_DIR/compiled
JAVA_CODE_DIR=$(cd $BOOTSTRAP_DIR/../ethclient/src/main/java/; pwd)

mkdir -p $COMPILED_DIR

$WEB3J solidity generate --solidityTypes -b $COMPILED_DIR/Network.bin -a $COMPILED_DIR/Network.abi -o $JAVA_CODE_DIR -p fluence.ethclient
