# Webassembly virtual machines perfomance comparison

Fluence computation machine[] is based on Webassembly executor. Essentially, WebAssembly is just a specification of an abstract stack-based virtual machine which leaves lots of freedom for the actual implementation. There are two types of virtual machines: compile-based and interpreter-based. The first type VM works like a classic interpreter: validate Wasm code, take one Wasm instruction and execute it without  


## Tests description

- tests description from Readme with chosen parameters

## Method of test

- which vm has been investigated
- how each vm has been compiled
- amazon m4.xlarge
- tests count
- why we have chosen a solid time for execution (JVM warmup and so on)
- link to cloned repos with VMs
- link to bencher script

## Test results

- pictures with results
- description of results

## Conclusion

- Why we have chosen asmble?
    - asmble is one of the most perfomance Wasm VM
    - asmble based on jvm - no cost for IPC

- we are planning to improve asmble (f.e. increase perfomance of SVD compressions tests and memory access by raw byte array)

