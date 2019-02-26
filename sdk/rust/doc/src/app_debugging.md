# Fluence app debugging

## Wasm logger usage

There is a few debugging capabilities for Wasm program. The Fluence network provides a possibility to so-called print-debugging. It can be included by specifying `wasm_logger` feature of the sdk. 

The logger is implemented as a logging facade for crate [`log`]. It means that all `log` crate can be used as usual. Let's review it by example of [hello-world2](https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-world2/app-2018) application with simple logging:

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
 
Generally it is better to initialize logger in the `init` function. And also note that `logger` designed only for Wasm environment and Fluence WasmVm. Don't use it for other targets and virtual machines. But if is is needed to create a project with `logger` either for Wasm target or other architectures conditional compilation can be used: 

```Rust
fn init() {
    if cfg!(target_arch = "wasm32") {
        logger::WasmLogger::init_with_level(log::Level::Info).unwrap();
    } else {
        simple_logger::init_with_level(log::Level::Info).unwrap();
    }
}
```

It is important to note that by default debugging capabilities is disabled on the Fluence network because of verification game process (you can find more information about it in [our paper](tbd)).

## Usage of Scala runner

Sometimes it needs not only debug output but a possibility to run resulted Wasm app with some different inputs. It can be done by using so-called runner written on Scala (because it uses a `WasmVm` implementation that written on Scala). 

An example of runner for `hello-world2` can be found [here](https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-world2/runner). It can be used a template for other runner realizations. These runners also support logging unless `loggerRegister` key in settings equals to 0 (for `hello-world2` app settings can be found [here](https://github.com/fluencelabs/fluence/blob/master/vm/examples/hello-world2/runner/src/main/resources/reference.conf)).
