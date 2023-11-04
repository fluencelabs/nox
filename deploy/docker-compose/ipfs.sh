#! /usr/bin/env sh

ipfs config --json API.HTTPHeaders.Access-Control-Allow-Origin '["*"]'
ipfs config --json API.HTTPHeaders.Access-Control-Allow-Methods '["PUT", "POST"]'
ipfs config --json Addresses.API '"/ip4/0.0.0.0/tcp/5001"'
ipfs bootstrap rm --all
