# Best practices for backend applications

## Request routing

Fluence does not provide a sane request routing yet – only one function can be marked as an entry point. This means that each developer has to independently decide on an input argument format, parse this argument in the entry point function, and call other functions based on the passed action selector.

One option would be to use JSON serialization (for example, provided by [serde](https://github.com/serde-rs/serde) and [serde_json](https://github.com/serde-rs/json) crates) to wrap input data. Once the input is parsed, the code in the entry point function can perform manual routing:

```Rust
use fluence::sdk::*;

use serde_json::{json, Value};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub action: String,
    pub player_name: String,
}

#[invocation_handler]
fn handle_request(req: String) -> String {
  let value: Value = serde_json::from_str(req.as_str())?;
  let request: Request = serde_json::from_value(value.clone())?;
  
  match request.action.as_str() {
    "create_game" => {
      // create a new game
    }
    
    ...
    
    "move" => {
      // make a player's move
    }
  }
}
```

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
