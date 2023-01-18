#!/bin/sh

# This script builds all subprojects and puts all created Wasm modules in one dir
marine --version
marine build --release

mkdir -p artifacts
rm -f artifacts/*
cp target/wasm32-wasi/release/file_share.wasm artifacts/
