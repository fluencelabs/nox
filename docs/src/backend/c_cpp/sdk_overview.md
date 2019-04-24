# SDK overview

## Compilation overview

There are two main requirements for provided Webassembly modules:
- they must have three export functions: `allocate`, `deallocate`, `invoke`
- they don't have other import functions except `write` and `flush` from logger module

You can find more detailed info about requirements in [internals](../internals.md) section of this guide. But compilation of C/C++ to Webassembly often generates modules with several syscall imports. We have tried to simplify the compilation process for C/C++ as much as possible.

There are few possible ways of compilation of C/C++ code to Webassembly, and two of them are most suitable for our platform:
- by wasm32-unknwon-wasi target
- by wasm32-unknown-unknown target

### wasm32-unknwon-wasi

Compilation with WASI-sysroot is quite simple:
 
1. WASI sysroot could be obtained by compilation of [wasi-sysroot](https://github.com/CraneStation/wasi-sysroot) or could be done by downloading and installing it from [releases](https://github.com/CraneStation/wasi-sdk/releases) (we recommend to use the latest one).

2. Given WASI in /opt/wasi-sdk compilation could be done by
```bash
~ $ clang --sysroot=/opt/wasi-sdk/ --target=wasm32-unknown-wasi -o module.wasm -nostartfiles -fvisibility=hidden -Wl,--no-entry,--export=allocate,--export=deallocate,--export=invoke,--allow-undefined -- *.c
```

There `--export` directives for linker manage it to make three functions exported from module and `--allow-undefined` used for make it possible to import `write` and `flush` functions.

### wasm32-unknwon-unknown

Compilation for wasm32-unknwon-unknown is more complicated and suggests building sysroot first:

1. First of all it needs to install the latest stable llvm 8.x that supports Webassembly. On Ubuntu it could be done by following commands (more detailed overview of llvm installation could be found [here](http://apt.llvm.org)):
```bash
# update sources with latest llvm mirror
~ $ echo "deb http://apt.llvm.org/bionic/ llvm-toolchain-bionic main" >> /etc/apt/sources.list.d/llvm.list && \
~ $ echo "deb-src http://apt.llvm.org/bionic/ llvm-toolchain-bionic main" >> /etc/apt/sources.list.d/llvm.list && \

# retrieve the archive signature
~ $ wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | apt-key add - && \

~ $ apt-get update

# install all key llvm packages
~ $ apt-get install -y clang-8 lldb-8 lld-8 libllvm-8-ocaml-dev libllvm8 llvm-8 llvm-8-dev llvm-8-doc llvm-8-examples llvm-8-runtime    clang-8 clang-tools-8 clang-8-doc libclang-common-8-dev libclang-8-dev libclang1-8 clang-format-8 python-clang-8 libc++-8-dev libc++abi-8-dev

# update environment variable with path to newly installed llvm
~ $ echo PATH="/usr/lib/llvm-8/bin:${PATH}"
~ $ echo LD_LIBRARY_PATH="/usr/lib/llvm-8/lib:${LD_LIBRARY_PATH}"
```

2. Configure and build musl (it is a C standard library)

```bash
# create directory for builds
~ $ mkdir ~/build && cd build

# clone our special version of musl
~ $ git clone https://github.com/fluencelabs/musl.git && cd musl

# configure musl build for wasm32-unknown-unknown-wasm target
~ $ ./configure CC=clang CFLAGS="--target=wasm32-unknown-unknown-wasm -O3" --prefix=/sysroot --enable-debug wasm32

# build musl in 8 threads
~ $ make -C /build/musl -j 8 install CROSS_COMPILE=llvm-

# make a directory for our sysroot
~ $ mkdir /opt/wasm-sysroot

# copy compiled musl to it
~ $ cp /src/musl/arch/wasm32/libc.imports /opt/wasm-sysroot/lib/
```

3. Build compiler-rt
 
```bash
~ $ cd ~/build
git clone https://github.com/llvm-mirror/compiler-rt.git && cd compiler-rt

# configure compiler-rt with cland for wasm32-unknown-unknown-wasm target
~ $ CC=clang cmake -DCMAKE_SYSROOT=/sysroot -DCOMPILER_RT_DEFAULT_TARGET_TRIPLE=wasm32-unknown-unknown-wasm -DCMAKE_C_COMPILER_WORKS=1 --target /src/compiler-rt/lib/builtins

# build compiler-rt
~ $ make

# copy compiler compiler-rt to clang libs
~ $ cp lib/*/libclang_rt.builtins-*.a /usr/lib/llvm-8/lib/clang/8.0.0/lib/
```

Finally, we have newly builded sysroot and could compile C/C++ code to Wasm using similar command as for WASI target:
```bash
~ $ clang --sysroot=/opt/wasm-sysroot --target=wasm32-unknown-unknown -o module.wasm -nostartfiles -fvisibility=hidden -Wl,--no-entry,--export=allocate,--export=deallocate,--export=invoke,--allow-undefined -- *.c
```

## Creating a Hello World backend app for Fluence

For a backend to be compatible with the Fluence network, it should follow aforementioned conventions to let Fluence nodes run your code correctly. To simplify the application process creating, we have developed the С/С++ SDK, dockerfile which includes all build internals, and the template project for C and C++. Let's learn how to use it!

### Building app written on C

The most simplest way of building application on C is using our template project that could be downloaded [here](https://github.com/fluencelabs/c-template). It includes sdk, Makefile and main.c file with `invoke` function defined. For simplicity С/С++ SDK is distributed as source code files not a binary. 

Let's open main.c in your favorite editor and write some code:

```C++
#include "sdk/allocator.h"
#include "sdk/logger.h"
#include <string.h>

const char *const greeting = "Hello world! From ";
const int RESPONSE_SIZE_BYTES = 4;

char *invoke(const char *str, int length) {
    const size_t greeting_length = strlen(greeting);
    const size_t response_length = length + greeting_length;

    char *response = (char *)allocate(response_length + RESPONSE_SIZE_BYTES);

    wasm_log(str, length);

    // (1)
    for(int i = 0; i < RESPONSE_SIZE_BYTES; ++i) {
        response[i] = (response_length >> 8*i) & 0xFF;
    }

    // (2)
    memcpy(response + RESPONSE_SIZE_BYTES, greeting, greeting_length);
    memcpy(response + RESPONSE_SIZE_BYTES + greeting_length, str, length);

    return response;
}
```

This invoke function receives pointer to some string and its length and append it to `Hello world! From ` string. To return result from the invoke function it needs to prepend it with its length written in little endian. In this example it is done in (1) and then in (2) the rest of result is copying.

This app could be built either with docker by
```bash
~ $ docker-compose up
```
or by Makefile
```bash
~ $ make CC=<path_to_clang> SYSROOT=<path_to_sysroot> TARGET_TRIPLE=<path_to_target_triple>
```

The complete Hello World example written on C could be found [here](https://github.com/fluencelabs/tutorials/tree/master/hello-world/app-logger-c).

### Building app written on C++

С++ application could be created in a similar ways as a C by using this [template](https://github.com/fluencelabs/cpp-template).

Let's open main.cpp in your favorite editor and write some code:

```C++
#include "sdk/sdk.h"
#include <string.h>

const std::string greeting = "Hello world! From ";

extern "C" char *invoke(char *str, int length) {
    const std::string request = sdk::read_request<std::string>(str, length);
    const std::string response = greeting + request;

    sdk::wasm_log(response);

    return sdk::write_response(response);
}
```

In this example we have some excess allocation (while transforming a raw supplied string to std::string and while appending two strings) and to reduce this overhead scheme from C application could be used.

The complete Hello World example written on C++ could be found [here](https://github.com/fluencelabs/tutorials/tree/master/hello-world/app-logger-cpp).
