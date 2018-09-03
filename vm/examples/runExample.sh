#!/usr/bin/env bash

exampleName=$1
projectDir=$2
scalaVer=$3

exampleFolder="$projectDir/vm/examples/$exampleName"

if [ ! -d "$exampleFolder" ]; then
  printf "'$exampleFolder' folder doesn't exist\n"
  exit 1;
fi

printf "    Compiling Rust to Wasm.\n"

docker run --rm -w /work -v "$exampleFolder:/work" tomaka/rustc-emscripten \
    cargo +nightly build --target wasm32-unknown-unknown --release

printf "    Build from WASM code executable WasmVM jar\n"

sbt "vm-$exampleName"/assembly

printf "    Run example $exampleName.jar\n"

java -jar "$exampleFolder/target/scala-$scalaVer/$exampleName.jar" "$exampleFolder/target/wasm32-unknown-unknown/release/$exampleName.wasm"