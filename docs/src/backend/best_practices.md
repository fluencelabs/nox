# Best practices

## Memory management in app

Please pay attention to module memory usage - each `app` can have maximum 4Gb of heap. But in many situations `app` can have public API that explicitly or indirectly consumes module memory (e.g., by saving some user data). So attackers can make a DoS attack to such `app`. Usually, proper memory management can be reached by using fix-size collections like [ArrayDeque](https://github.com/andylokandy/arraydeque). In future Fluence backend SDK will provide some other collections for that.

One example of careful memory management can be found in [tic-tac-toe](https://github.com/fluencelabs/fluence/tree/master/vm/examples/tic-tac-toe).

## Shrinking app code size

There are some techniques for reducing size of a generated `app`. Most of them can be found in this excellent [chapter](https://rustwasm.github.io/book/reference/code-size.html) of `Rust and WebAssembly` book. But as for the Fluence network to reduce the size of binary as much is possible `no_std` crate without SDK can be used. The example of `hello-world` implemented without the SDK can be found in [Fluence backend SDK overview](sdk_overview.md).

## Panics

Using of `unwrap`, `expect` and others like these are possible in an `app`. Each of them leads to an exception that will catch in the `VM wrapper`. A error message of such exception won't be much informative in this case (like this `{"Error":{"code":"TrapError","message":"Function invoke with args: List(1117056, 53) was failed","cause":"Function invoke with args: List(1117056, 53) was failed"}}`), but `app` will be in a consistent state and could process other requests.
