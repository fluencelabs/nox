#!/bin/bash

pushd bootstrap

npm install
pkill -f ganache # kill Ganache if it was running already
npm run ganache > /dev/null
npm run migrate

echo "Current Ethereum account is" $(npm run --silent getEthAccount)

popd