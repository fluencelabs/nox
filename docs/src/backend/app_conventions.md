# Fluence backend conventions

## App lifecycle

The Fluence network backend infrastructure is based on Scala and Rust. Each computation unit node in the network has a `VM wrapper` written on Scala that interacts with a backend `App` written on Wasm and published to the network by developers. To be able to run supplied Wasm code [Asmble](https://github.com/fluencelabs/asmble) is used. It compiles supplied Wasm cdoe to a JVM class that then loaded by `VM wrapper`. All other interactions between `App` and `VM wrapper` occurs in `VM wrapper` process address space without any inter-process communications.

A `main` module is invoked by `VM wrapper` according to the following scheme:

1. A `client-side` send a request to `App` as a byte array.

2. `VM wrapper` call `allocate` function of `main` Wasm module with a size of the array.

3. `VM wrapper` writes the array to the module memory.

4. `VM wrapper` call `invoke` function from `main` module with the address returned from `allocate` function and the array size.

5. `VM wrapper` synchronously waits of `invoke` result. After receiving a `pointer` from it, reads 4 bytes (that represents `size` of a byte array) and then reads `size` bytes from `pointer + 4` offset (`result`).

6. `VM wrapper` call `deallocate` function of `main` Wasm module with the received pointer.

7. Finally, `result` as a byte array is sent to a `client-side`.

## Fluence App conventions

There are several restriction and conventions that each supplied `App` has to be met:

1. `App` can consist of several Wasm modules with different names, but only one of them (let's call it `main` and all other as `sidemodules` according to [emcscripten](https://github.com/emscripten-core/emscripten/wiki/Linking#overview-of-dynamic-linking)) can be called from `user-side`. This `main` module MUST don't have the module name section. This requirement is based on the fact that according to the Wasm specification module name is optional, and now there is no a possibility to add it to a generated Wasm binary by default `rust` compiler.

2. Each `main` module MUST have three export (in terms of the Wasm specification) functions with names `invoke`, `allocate` and `deallocate`.

3. `invoke` function is used as the `main` module handler function. It means that all client-side requests are routed to it. The exactly signature of this function MUST be `(func (export "invoke") (param $buffer i32) (param $size i32) (result i32))` in wast representation. It receives two i32 params that represent a pointer to supplied argument and its size. If `client-side` send an empty byte buffer `invoke` SHOULD be called with two nulls (it means that according to Fluence protocol implementation honest nodes call `invoke` with nulls but malicious nodes can do anything). This function has to return a pointer to result that MUST have the next structure in memory: `| size (4 bytes; little endian) | result buffer (size bytes) |`. This convention is based on the fact that Wasm function can return only one value of i32, i64, f32, f64, i128 but there both pointer and size should be returned.

4. `allocate` function MUST have the next signature `(func (export "allocate") (param $size i32) (result i32))` in wast representation. It MUST return a pointer as i32 to a module memory region long enough to hold `size` bytes.

5. `deallocate` function MUST have the next signature `(func (export "deallocate") (param $address i32) (param $size i32) (return))`. It is called by `VM wrapper` with a pointer to a memory region previously allocated through `allocate` function and its size. This function SHOULD free this memory region.
