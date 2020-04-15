#!/bin/bash

set -m

/janus-server &
SERVER=$!


/janus-ipfs  &
IPFS=$!
