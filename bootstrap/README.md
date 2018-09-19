# First
`npm install`

# Running Scala tests

## How to generate abi & Java wrapper
`npm run generate-abi`

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