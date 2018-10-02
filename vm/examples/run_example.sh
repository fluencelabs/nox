#!/usr/bin/env bash

exampleName=$1
projectDir=$2
scalaVer=$3

exampleFolder="$projectDir/vm/examples/$exampleName"
prefix="<run_example.sh>"

if [ ! -d "$exampleFolder" ]; then
  printf "$prefix [ERROR]: '$exampleFolder' folder doesn't exist\n"
  exit 1;
fi

printf "$prefix Compiling Rust to Wasm.\n"

docker run --rm -w /work -v "$exampleFolder:/work" tomaka/rustc-emscripten \
    cargo +nightly build --target wasm32-unknown-unknown --release

if [ $? -ne 0 ]; then
   printf "$prefix [ERROR]: docker command finish with error \n"
   exit 1;
fi

printf "$prefix Build from WASM code executable WasmVM jar\n"

sbt "vm-$exampleName"/assembly

printf "$prefix Run example $exampleName.jar\n"

java -jar "$exampleFolder/target/scala-$scalaVer/$exampleName.jar" "$exampleFolder/target/wasm32-unknown-unknown/release/$exampleName.wasm"
