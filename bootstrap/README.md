# First
`npm install`

# Running Scala tests

## How to generate abi 
please note, comma in the `allow-paths` is crucial.
`solc --overwrite --bin --abi -o fluence/bootstrap/contracts/ =fluence/bootstrap/node_modules/ --allow-paths fluence/bootstrap/node_modules/, fluence/bootstrap/contracts/Deployer.sol`

## How to generate java contract abi
`web3j solidity generate fluence/bootstrap/contracts/Deployer.bin fluence/bootstrap/contracts/Deployer.abi -o fluence/ethclient/src/main/java/fluence/ethclient/ -p fluence.ethclient`

## Running private blockchain with docker
See [guide on running private ethereum blockchain](private_ethereum.md)

## Deploying contract to blockchain
`node_modules/.bin/truffle migrate`

Then, copy & paste contract's address and your wallet into `ContractSpec` and run tests:

`sbt ethclient/test`

# Running nodejs tests
## Run private blockchain via ganache
`npm run ganache`

## Run tests
`npm test`