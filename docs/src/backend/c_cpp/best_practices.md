# Best practices for C/C++ backend applications

## Request routing

Fluence does not provide a sane request routing yet – only one function can be marked as an entry point. This means that each developer has to independently decide on an input argument format, parse this argument in the entry point function, and call other functions based on the passed action selector.

One option would be to use JSON serialization (for example, provided by [parson](https://github.com/kgabis/parson)) to wrap input data.

## Syscall imports

Sometimes especially by including some libraries (like `stdio.h`) imports of syscall could appear in compiled wasm binary. For checking the appearance of such imports, it just needs to translate wasm (binary format) to wast (test format) ([wabt](https://github.com/WebAssembly/wabt) could be used) and check manually for imports. All possible imports are these two needed for logger:
```
(import "logger" "write" (func $__write (type 0)))
(import "logger" "flush" (func $__flush (type 7)))
```

Almost always imports of syscall could be replaced in text format to empty function.

## Carefully tracking compiler and linker warnings

Generally, it is good to investigate all tracking compiler and linker warnings and try to get rid of them. But especially it is important during compilation to Webassembly. One of these seemingly innocuous warnings could look like this:

```bash
wasm-ld: warning: function signature mismatch: qsort
>>> defined as (i32, i32, i32, i32) -> i32 in geo.o
>>> defined as (i32, i32, i32, i32) -> void in /opt/wasi-sdk/share/sysroot/lib/wasm32-wasi/libc.a(qsort.o)
```

It could happen because of lack of `stdlib.h` in includes and manages compiler to generate an unnecessary `drop` instruction since the compiler will use the different signature of this function. And finally, such binary is invalid and couldn't run on Fluence.

## Shrinking app code size

In most cases, there is no need to worry about the size of the generated WebAssembly code. However, there some common techniques for reducing the WebAssembly package size:
- using `-O3` option for aggressive optimization 
- using `--strip-all` options of linker for strip all symbols from generated binary
- not using `double float` and all functions that receive it as a parameter because it generates a lot of wasm instructions and generates imports to compiler-rt lib.
