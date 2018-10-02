#!/bin/bash
docker run -p 26657:26657 -p 26660:26660 -v $PWD/statemachine:/statemachine -v $PWD/tendermint:/tendermint statemachine/statemachine:latest
