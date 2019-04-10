# Developing the backend app on AssemblyScript

Let's build Hello World app for Fluence.

## Requirements

- installed `npm`

## Initialize project

1. Create a new directory for a project
2. Run `npx fluencelabs/fl-init` from directory.
3. Wait one minute. Enter 'y' when asked. 
4. This script will initialize project, that we can already compile to WebAssembly and deploy into Fluence system.

## Creating a Hello World backend for Fluence

All we need for now is to change `handle` function in `main.ts` file: 
```js
// main handler for an application
export function handler(request: string): string {
  return "";
}
```

Change empty string with `return "Hello, " + request + "!"` and that's it, the app is ready.

## Build `wasm` file

1. Run `npm run flbuild` in the project directory.
2. File `build/optimized.wasm` will be created.
3. You can publish this file like in [Publishing the backend app](publish.md) tutorial.


## Advanced

For deeper understanding let's open `assembly/index.ts` file. We can see functions, that are gateways from outer VM to written code:
```js
...
// VM wrapper will put requests to memory through this function
export function allocate(size: usize) :i32 {
  return memory.allocate(size);
}

// VM wrapper will deallocate response from memory after handling it
export function deallocate(ptr: i32, size: usize): void {
  memory.free(ptr);
  // use 'memory.reset()' for reset memory after every request
}

// VM wrapper calls this function with a pointer on request in memory.
// Returns pointer on a response.
export function invoke(ptr: i32, size: i32): i32 {
    // this function will parse a request as a string and return result string as a pointer in memory
    // you can look on other functions in 'assemblyscript-sdk' library to handle own types of requests and responses
    return loggedStringHandler(ptr, size, handler, log);
}
```

Function `loggedStringHandler` in `invoke` use `handler` to handle incoming messages and returning response. There are some other methods for interacting with requests and responses in different formats. You can look at them here [AssemblyScript SDK](https://github.com/fluencelabs/assemblyscript-sdk/blob/master/assembly/index.ts) 

## Examples
[Dice game](https://github.com/fluencelabs/tutorials/tree/master/dice-game/backend-as)
[Multimodule dice game with connected DB written on Rust](https://github.com/fluencelabs/tutorials/tree/master/dice-game/backend-db-as)

## Limitations of AssemblyScript
It is important to note that AssemblyScript is NOT TypeScript. There are several important features that are missing notably:
- interfaces
- use of untyped variables or the any type
- JSON.parse/JSON.stringify
- virtual methods (the method of the compile time type will always be called)
