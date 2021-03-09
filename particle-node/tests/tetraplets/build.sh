#!/bin/sh

# This script builds all subprojects and puts all created Wasm modules in one dir
cargo update
fce build --release

mkdir -p artifacts
rm -f artifacts/*
cp target/wasm32-wasi/release/tetraplets.wasm artifacts/
