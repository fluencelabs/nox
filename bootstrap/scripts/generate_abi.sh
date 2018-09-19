#!/bin/bash -e

command -v web3j >/dev/null 2>&1 || { 
    echo >&2 "web3j should is not installed. See https://web3j.readthedocs.io/en/latest/command_line.html" 
    exit 1 
}

command -v solc >/dev/null 2>&1 || { 
    echo >&2 "solc should is not installed. See https://solidity.readthedocs.io/en/latest/installing-solidity.html" 
    exit 1 
}

WEB3J=$(command -v web3j)
SOLC=$(command -v solc)
BOOTSTRAP_DIR=$(pwd)
CONTRACTS_DIR=$BOOTSTRAP_DIR/contracts
COMPILED_DIR=$CONTRACTS_DIR/compiled
NODE_MODULES_DIR=$BOOTSTRAP_DIR/node_modules
JAVA_CODE_DIR=$(cd $BOOTSTRAP_DIR/../ethclient/src/main/java/; pwd)

echo "compiling Deployer.sol"
mkdir -p $COMPILED_DIR
# Comma in '--allow-paths' is intended and crucial, keep it!
$SOLC --overwrite --bin --abi -o $COMPILED_DIR openzeppelin-solidity=$NODE_MODULES_DIR/openzeppelin-solidity --allow-paths $NODE_MODULES_DIR, $CONTRACTS_DIR/Deployer.sol

echo "generating a Java wrapper for Deployer.sol"
$WEB3J solidity generate --solidityTypes $COMPILED_DIR/Deployer.bin $COMPILED_DIR/Deployer.abi -o $JAVA_CODE_DIR -p fluence.ethclient