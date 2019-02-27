# Fluence backend SDK overview

Fluence SDK consists of two crates: `main` and `macro`. The first one is used for all memory relative operations and logging (please see [app debugging](./app_debugging.md) section for more information). The second one contains a procedural macro to simplify the entry function's signature. These crates can be used separately but the more preferred way is using the global `fluence` crate that reexport all others. In `Rust 2018 edition` it can be done by simply adding `use fluence::sdk::*` to the source.

## Writing app without SDK

For a start, lets review how a simple `hello-world` `app` is made without Fluence SDK on pure Rust. From [fluence backend conventions](app_conventions.md) it follows that each `app` must have a `main` module with three export functions. Keeping in mind restrictions to their signatures, a basic structure of `main` module can look like that:

 ```Rust
 #[no_mangle]
 pub unsafe fn invoke(ptr: *mut u8, len: usize) -> usize {
     ...
 }
 
 #[no_mangle]
 pub unsafe fn allocate(size: usize) -> NonNull<u8> {
     ...
 }
 
 #[no_mangle]
 pub unsafe fn deallocate(ptr: *mut u8, size: usize) {
     ...
 }
 ```

Note that `#[no_mangle]` and `pub unsafe` parts of function signature manage function to be exported from a Wasm module (more information can be found in this [discussion](https://internals.rust-lang.org/t/precise-semantics-of-no-mangle/4098)).
 
To implement `hello-world` example from [quick start](TODO) these functions can be implemented like this:
 
```Rust
#[no_mangle]
pub unsafe fn invoke(_ptr: *mut u8, _len: usize) -> usize {
    let result = "Hello, world!";
    
    let result_len = result.len();
    let total_len = result_len
        .checked_add(RESULT_SIZE_BYTES)
        .ok_or_else(|| MemError::new("usize overflow occurred"))?;

    // converts array size to bytes in little-endian
    let len_as_bytes: [u8; RESULT_SIZE_BYTES] = mem::transmute((result_len as u32).to_le());

    // allocates a new memory region for the result
    let result_ptr = alloc(NonZeroUsize::new_unchecked(total_len))?;

    // copies length of array to memory
    ptr::copy_nonoverlapping(
        len_as_bytes.as_ptr(),
        result_ptr.as_ptr(),
        RESULT_SIZE_BYTES,
    );

    // copies array to memory
    ptr::copy_nonoverlapping(
        result.as_ptr(),
        result_ptr.as_ptr().add(RESULT_SIZE_BYTES),
        result_len,
    );

    result_ptr as usize
}

#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    let non_zero_size = NonZeroUsize::new(size)
        .unwrap_or_else(|| panic!("[Error] Allocation of zero bytes is not allowed."));
        
    let layout: Layout = Layout::from_size_align(non_zero_size.get(), mem::align_of::<u8>())?;
    Global.alloc(layout)
        .unwrap_or_else(|_| panic!("[Error] Allocation of {} bytes failed.", size))
}

#[no_mangle]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize) {
    let non_zero_size = NonZeroUsize::new(size)
        .unwrap_or_else(|| panic!("[Error] Deallocation of zero bytes is not allowed."));
    let layout = Layout::from_size_align(non_zero_size.get(), mem::align_of::<u8>())?;
    Global.dealloc(ptr, layout);
}
``` 

## Fluence SDK usage

From the example above it can be seen that `allocate` and `deallocate` functions serve only utility purpose and are normally used only by VM wrapper. These functions aren't any that most of developers would want to implement in their `app`. Fluence backend SDK provides `fluence::memory::alloc` and `fluence::memory::dealloc` functions also based on [GlobalAlloc](https://doc.rust-lang.org/beta/std/alloc/trait.GlobalAlloc.html). They exported by default after including the sdk and can be disabled by specifying the `no_export_allocator` feature of the sdk.

The `invoke` function can be also simplified by using the Fluence backend SDK that provides `fluence::memory::write_result_to_mem` and `fluence::memory::read_input_from_mem_` in this way:

```Rust
#[no_mangle]
pub unsafe fn invoke(_ptr: *mut u8, _len: usize) -> usize {
    let test_str = "Hello, world!";
    fluence::memory::write_result_to_mem(test_str)
        .unwrap_or_else(|_| {
            panic!("[Error] Putting the result string into a raw memory was failed")
        })
        .as_ptr() as usize
}
```

### Invocation handler to the rescue

The example above can be simplified a little bit more using procedural macros `invocation_handler` as `hello-world` example mentioned in [quick start](TODO) example:
 
```Rust
use fluence::sdk::*;

#[invocation_handler]
fn greeting(name: String) -> String {
    format!("Hello, world! From user {}", name)
}
```

Internally this macros creates a new function `invoke` that converts a raw argument to appropriate format, calls `f` and then converts its result via `memory::write_result_to_mem` from `fluence_sdk_main`. The following listing shows how this macros is expanded:
 
```Rust
use fluence::sdk::*;

fn greeting(name: String) -> String {
    format!("Hello, world! From user {}", name)
}

#[no_mangle]
pub unsafe fn invoke(ptr: *mut u8, len: usize) -> std::ptr::NonNull<u8> {
    let arg = memory::read_input_from_mem(ptr, len);
    let arg = String::from_utf8(arg).unwrap();
    let result = greeting(arg);
    memory::write_result_to_mem(result.as_bytes()).expect("Putting result string to memory has failed")
}
```

To use this macro with a function `f` some conditions have to be satisfied:

1. `f` mustn't have more than one input argument.

2. `f` mustn't be `unsafe`, `const`, generic, have custom abi linkage or variadic param.

3. The type of `f` input (if it present) and output parameters have to be one of {String, Vec<u8>} set.

4. `f` mustn't have the name `invoke`.

For troubleshooting and macros debugging [cargo expand](https://github.com/dtolnay/cargo-expand) can be used. 

The macro also has an `init_fn` attribute that can be used for specifying initialization function name. This function will be called only in the first `invoke` function call. It can be used like this:

```Rust
use fluence::sdk::*;

fn init() {
    ...
}

#[invocation_handler(init_fn = init)]
fn greeting(name: String) -> String {
    format!("Hello from Fluence to {}", name)
}
```

This is expanded to

```Rust
use fluence::sdk::*;

fn init() { 
    ...
}

fn greeting(name: String) -> String {
    format!("Hello from Fluence to {}", name)
}

static mut IS_INITED: bool = false;

#[no_mangle]
pub unsafe fn invoke(ptr: *mut u8, len: usize) -> std::ptr::NonNull<u8> {
    if !IS_INITED { 
        init();
        unsafe { IS_INITED = true; }
    }
    let arg = memory::read_input_from_mem(ptr, len);
    let arg = String::from_utf8(arg).unwrap();
    let result = greeting(arg);
    memory::write_result_to_mem(result.as_bytes()).expect("Putting result string to memory has failed")
}
```

Example of usage `invocation_handler` with `init_fn` attribure can be found [here](https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-world2/app-2018).

## Rust edition 2015

To use Fluence SDK with `Rust 2015 edition` please import it like this: 

```Rust
#![feature(custom_attribute)]
extern crate fluence;

use fluence::sdk::*;
```

Example of `hello-world2` `app` on `Rust edition 2015` can be found [here](https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-world2/app-2015).
