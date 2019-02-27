# Best practices

## Memory management in app

Please pay attention to module memory usage - each `app` can have maximum 4Gb of heap. But in many situations `app` can have public API that explicitly or indirectly consume module memory (e.g. by saving some user data). So users can make a DoS attack to such app. Usually proper work with memory can be reach by using fix-size collections like [ArrayDeque](https://github.com/andylokandy/arraydeque). In future Fluence SDK will provide some other collections for that.

One example of careful memory management can be found in [tic-tac-toe](https://github.com/fluencelabs/fluence/tree/master/vm/examples/tic-tac-toe) example `app`.

## Shrinking app code size

There are some techniques that can reduce size of generated `app`. Most of them can be found in this nice [chapter](https://rustwasm.github.io/book/reference/code-size.html) of `Rust and WebAssembly` book. But as for the Fluence network to reduce the size of binary as much is possible `no_std` crate without SDK can be used. The example of `hello-world` implemented without the sdk can be found in [Fluence SDK overview](sdk_overview.md).

## Other

- don't use panic! and methods that lead to it (expect, unwrap) in your code.

- avoid using unsafe operations except these that already in Fluence SDK.
