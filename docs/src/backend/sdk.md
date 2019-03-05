# Backend SDK overview

Fluence backend SDK consists of two crates: `main` and `macro`. The `main` crate is used for all memory relative operations and logging, while the `macro` crate contains the macro to simplify entry point functions. These crates can be used separately but the preferred way is to use the global `fluence` crate which combines all the others. 

In Rust 2018 this can be done by adding [Fluence SDK](https://crates.io/crates/fluence) as a dependency, and then adding `use fluence::sdk::*` to Rust sources.

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

### Direct entry point function implementation

Another option is to not use Fluence SDK macro and implement the `invoke()` function manually:

```Rust
use fluence::sdk::*;

fn greeting(name: String) -> String {
    format!("Hello, world! -- {}", name)
}

#[no_mangle]
pub unsafe fn invoke(ptr: *mut u8, len: usize) -> std::ptr::NonNull<u8> {
    let arg = memory::read_input_from_mem(ptr, len);
    let arg = String::from_utf8(arg).unwrap();
    
    let result = greeting(arg);
      
    memory::write_result_to_mem(result.as_bytes()).expect("Memory write has failed")
}
```

Here you can see that the entry point function takes the pointer to the byte array and the length of the byte array. Then, it uses `fluence::memory::read_input_from_mem_` to read this data into the string argument, and calls the greeting function. Once the greeting function has returned a result, this result is written back to the WebAssembly memory with the help of `fluence::memory::write_result_to_mem` function.

To learn more about internals of Fluence backend applications, please consult [backend API conventions](./conventions.md).

## Rust 2015

To use Fluence SDK with Rust 2015 import it like this: 

```Rust
#![feature(custom_attribute)]
extern crate fluence;

use fluence::sdk::*;
```

Example Rust 2015 application can be found [here](https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-world2/app-2015).
