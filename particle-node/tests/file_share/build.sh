#!/bin/sh

# This script builds all subprojects and puts all created Wasm modules in one dir
cargo update --aggressive
fce build --release

mkdir -p artifacts
rm -f artifacts/*
cp target/wasm32-wasi/release/file_share.wasm artifacts/
