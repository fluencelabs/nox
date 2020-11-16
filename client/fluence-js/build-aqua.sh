#!/bin/bash

if [ -z "$1" ]; then
    echo "No path to an aquamarine crate"
    exit 1
fi

npm run build-wasm -- $1

# add a wasm file as a base64 string to typescript
base64 -w 0 pkg/index_bg.wasm > src/aqua/wasmBs64.ts && sed -i '1s/^/export const wasmBs64=\"/' src/aqua/wasmBs64.ts && echo '"' >> src/aqua/wasmBs64.ts