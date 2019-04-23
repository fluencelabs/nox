# Compilation overview


The main obstacle of compiling C/C++ to Webassembly is our requirement to resulted module not to have function imports.  

There are few possible ways of compilation of C/C++ code to Webassembly, but we will 
- by wasm32-unknown-unknown target
- by wasm32-unknwon


## Request routing


## Authentication

**TBD**

## Memory management

At the current moment, backend applications cannot use more than 4GB of WebAssembly memory. To avoid running out of memory, applications should carefully plan memory usage. For example, an open to the public multi-user game might employ a [ring buffer](https://en.wikipedia.org/wiki/Circular_buffer)-like data structure to evict [least recently created](https://en.wikipedia.org/wiki/Cache_replacement_policies#First_in_first_out_(FIFO)) user records first.

Crates such as [`arraydeque`](https://docs.rs/arraydeque/0.4.3/arraydeque/) or [`linked_hash_map`](http://contain-rs.github.io/linked-hash-map/linked_hash_map/) can be used to limit the number of stored entries.

Other policies such as [LRU](https://en.wikipedia.org/wiki/Cache_replacement_policies#Least_recently_used_(LRU)) eviction can be used as well. Another option would be to restrict the number of users the application can handle or accept state modifications made only by the limited subset of users with write permissions.

An illustration of vigilant memory management can be found in the [tic-tac-toe](https://github.com/fluencelabs/fun/tree/master/tic-tac-toe) example game.

## Shrinking app code size

In most cases, there is no need to worry about the size of the generated WebAssembly code. However, techniques for reducing the WebAssembly package size can be found in the [Rust and WebAssembly book](https://rustwasm.github.io/book/reference/code-size.html). Another option would be to avoid the use of the Fluence SDK and implement required API functions from scratch.

## Panics

Backend applications are allowed to panic, for example, if an `unwrap` or `expect` call fails. At the current moment, the returned error message will not be too informative in this case:
```
{
   "Error":{
      "code":"TrapError",
      "message":"Function invoke with args: List(1117056, 53) was failed",
      "cause":"Function invoke with args: List(1117056, 53) was failed"
   }
}
```

However, the application will remain in consistent state and will be able to process other requests.
