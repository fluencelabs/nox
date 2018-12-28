### Fluence Rust examples

Rust examples that can be compiled to Webassembly and run in Fluence.  

#### Prerequisites

Check that you've installed Java 8, `docker` and `docker` is running. Also you need nigthly `rust` compiler with wasm32-unknown-unknown target:

```shell
# download and install rustup
curl -sSf https://static.rust-lang.org/rustup.sh | sh

# install the latest nightly toolchain
~/.cargo/bin/rustup toolchain install nightly

# make shure that Rust is up to date
rustup update

# install the Webassembly target for Rust
rustup target add wasm32-unknown-unknown --toolchain nightly
```


#### Examples compiling

For example compilation simply run

        sbt vm-{EXAMPLE_NAME}/compile
        
from the project root folder.
