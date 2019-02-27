# Backend app debugging

## Wasm logger usage

There is a few debugging capabilities for Wasm program. The Fluence network provides a possibility to so-called print-debugging. It can be included by specifying `wasm_logger` feature of the sdk. 

The logger is implemented as a logging facade for crate [log](https://github.com/rust-lang-nursery/log). It means that all `log` crate possibilities can be used as usual. Let's review it by example of [hello-world2](https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-world2/app-2018) application with simple logging:

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
 
The easiest way to initialize logger is using the `init_fn` attribute of `invocation_handler` like in the example above.

Please also note that this `logger` designed only for Wasm environment and Fluence `WasmVm`. Don't use it for other targets and virtual machines. But if it needs to create a project with `logger` either for Wasm target and other architectures, a conditional compilation can be used: 

```Rust
fn init() {
    if cfg!(target_arch = "wasm32") {
        logger::WasmLogger::init_with_level(log::Level::Info).unwrap();
    } else {
        simple_logger::init_with_level(log::Level::Info).unwrap();
    }
}
```

It is important to note that by default debugging capabilities is disabled in the Fluence network because of verification game process (you can find more information about it in [our paper](TODO)).

## Usage of Scala runner

Sometimes it needs not only a debug output but a possibility to run resulted Wasm `app` with some different inputs. It can be done by using so-called runner written on Scala (because of it uses a `WasmVm` implementation that also written on Scala).

Let's a dig a little bit into `hello-world2` [runner]((https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-world2/runner)). It receives a path to a Wasm binary as the first CLI argument, then creates `vm` object that extends `WasmVm` trait. During this creation Wasm code is compiled by Asmble to JVM and loaded into VM. This trait has two public methods: `invoke` that manages requests to `app` and `getVmState` that computes a hash of significant inner state. In the following code snippet

```Scala
      inputFile <- EitherT(getWasmFilePath(args).attempt)
        .leftMap(e => InternalVmError(e.getMessage, Some(e)))
        
      vm ← WasmVm[IO](NonEmptyList.one(inputFile), "fluence.vm.debugger")
      
      initState ← vm.getVmState[IO]

      result1 ← vm.invoke[IO](None, "John".getBytes())
```

`inputFile` points to supplied path to wasm file. Then WasmVm instance is created with path and `fluence.vm.debugger` config. This config (for `hello-world2` `app` it can be found [here](https://github.com/fluencelabs/fluence/blob/master/vm/examples/hello-world2/runner/src/main/resources/reference.conf)) contains some useful settings that control some inner VM creation process a little bit:

  - `defaultMaxMemPages` - the maximum number of memory pages when a module doesn't specify it, each Wasm page according to the specification contains 65536 bytes (64 by default, 65536*64 = 4MB)

  - `loggerRegister` - if > 0, registers the logger Wasm module as 'logger' with specified number of memory pages, that allows to logs to stdout (0  by default in mainnet and 2 by default for runners)

  - `allocateFunctionName` - the name of function that should be called for allocation memory (`allocate` by default)

  - `deallocateFunctionName` - the name of function that should be called for deallocation of previously allocated memory by allocateFunction (`deallocate` by default)

  - `invokeFunctionName` - the name of the main module handler function (`invoke` by default)

Then the hash of internal state is computed by `vm.getVmState` and `invoke` from `app` is called by `vm.invoke`.
