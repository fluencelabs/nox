#!/bin/bash -e
npm run generate-all
echo "deployerAbi = $(cat contracts/compiled/Deployer.abi)"
echo "deployerHex = \"0x$(cat contracts/compiled/Deployer.bin)\""
