# Backend SDK overview

Fluence backend SDK consists of two crates: `main` and `macro`. The `main` crate is used for all memory relative operations and logging, while the `macro` crate contains the macro to simplify entry point functions. These crates can be used separately but the preferred way is to use the global `fluence` crate which combines all the others. 

In Rust 2018 this can be done by adding [Fluence SDK](https://crates.io/crates/fluence) as a dependency, and then adding `use fluence::sdk::*` to Rust sources.

## Rust 2015

To use Fluence SDK with Rust 2015 import it like this: 

```Rust
#![feature(custom_attribute)]
extern crate fluence;

use fluence::sdk::*;
```

Example Rust 2015 application can be found [here](https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-world2/app-2015).

## Entry point function

Each WebAssembly backend application deployed to the Fluence network is expected to provide a single entry point function named `invoke`. The easiest way to implement this function is to use the `invocation_handler` macro provided by the Fluence SDK:

```Rust
use fluence::sdk::*;

#[invocation_handler]
fn greeting(name: String) -> String {
    format!("Hello, world! -- {}", name)
}
```

If anything goes wrong, [cargo expand](https://github.com/dtolnay/cargo-expand) can be used for troubleshooting and macros debugging.  Keep in mind that the function which the macro is attached to should:

- not have more than one input argument and always return a value 
- not be unsafe, const, generic, or have custom abi linkage or variadic params
- have the input argument type (if present) and return type to be either `String` or `Vec<u8>`
- not use the `invoke` name, which is reserved by the Fluence SDK

The `invocation_handler` macro additionally allows to specify the initialization function:

```Rust
use fluence::sdk::*;

fn init() {
    // will be called just before the first `greeting()` function invocation
}

#[invocation_handler(init_fn = init)]
fn greeting(name: String) -> String {
    format!("Hello, world! -- {}", name)
}
```

The initialization function will be called only once and before any other code when the first transaction arrives. Therefore, it is a great place to put the code preparing the backend application.

## Debugging

**TBD**

<!-- ## App runner

Sometimes it needs not only a debug output but a possibility to run a compiled Wasm `app` with some different inputs. It can be done by using so-called runner written on Scala (because it uses a `WasmVm` implementation that also written on Scala).

Let's a dig a little bit into the `hello-world2` [runner]((https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-world2/runner)). It receives a path to a Wasm binary as the first CLI argument, then creates a `vm` object that extends `WasmVm` trait. During this creation, Wasm code is compiled by Asmble to JVM and loaded into VM. This trait has two public methods: `invoke` that manages requests to `app` and `getVmState` that computes a hash of significant inner state. In the following code snippet

```Scala
      inputFile <- EitherT(getWasmFilePath(args).attempt)
        .leftMap(e => InternalVmError(e.getMessage, Some(e)))
        
      vm ← WasmVm[IO](NonEmptyList.one(inputFile), "fluence.vm.debugger")
      
      initState ← vm.getVmState[IO]

      result1 ← vm.invoke[IO](None, "John".getBytes())
```

`inputFile` points to a supplied path to wasm file. Then WasmVm instance is created with the path to input file and `fluence.vm.debugger` config. This config (for the `hello-world2` `app` it can be found [here](https://github.com/fluencelabs/fluence/blob/master/vm/examples/hello-world2/runner/src/main/resources/reference.conf)) contains some useful settings that control some inner VM creation process a little bit:

  - `defaultMaxMemPages` - the maximum number of memory pages when a module doesn't specify it, each Wasm page according to the specification contains 65536 bytes (64 by default, 65536*64 = 4MB)

  - `loggerRegister` - if > 0, registers the logger Wasm module as `logger` with specified number of memory pages, that allows to logs to stdout (0  by default in mainnet and 2 by default for runners)

  - `allocateFunctionName` - the name of function that should be called for memory allocation (`allocate` by default)

  - `deallocateFunctionName` - the name of function that should be called for deallocation of previously allocated memory by `allocateFunction` (`deallocate` by default)

  - `invokeFunctionName` - the name of the main module handler function (`invoke` by default)

Then the hash of internal state is computed by `vm.getVmState` and `invoke` from `app` is called by `vm.invoke`.

## Wasm logger

To debug backend applications, the SDK provides 
There are a few debugging capabilities for Wasm program. The Fluence network provides a possibility to so-called print-debugging. It can be included by specifying the `wasm_logger` feature of the sdk. 

The logger is implemented as a logging facade for crate [log](https://github.com/rust-lang-nursery/log). It means that all `log` crate possibilities can be used as usual. Let's review it by the example of [hello-world2](https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-world2/app-2018) application with simple logging:

```Rust
use fluence::sdk::*;
use log::info;

fn init() {
    logger::WasmLogger::init_with_level(log::Level::Info).is_ok()
}

#[invocation_handler(init_fn = init)]
fn main(name: String) -> String {
    info!("{} has been successfully greeted", name);
    format!("Hello from Fluence to {}", name)
}
```
 
The easiest way to initialize the logger is using the `init_fn` attribute of `invocation_handler` like in the example above.

Please also note that this `logger` is designed only for Wasm environment and Fluence `WasmVm`. Don't use it for other targets and virtual machines. But if it needs to create a project with `logger` either for Wasm target and other architectures, a conditional compilation can be used: 

```Rust
fn init() {
    if cfg!(target_arch = "wasm32") {
        logger::WasmLogger::init_with_level(log::Level::Info).unwrap();
    } else {
        simple_logger::init_with_level(log::Level::Info).unwrap();
    }
}
```

Without this trick `app` that uses the logger can be compiled only for `wasm32-unknown-unknown` target. 

It is also important to note that by default debugging capabilities is disabled in the Fluence network because of verification game process (you can find more information about it in [our paper](TODO)). -->


