# Project status

At the current moment, the project is under heavy development, and the architecture described in the [overview](introduction/overview.md) is not covered completely by the deployed devnet. Below we have listed features that were already rolled out:

**+** Real-time processing layer: real-time clusters with built-in BFT consensus (Tendermint)  
**+** Secure metadata storage: real-time clusters state (Ethereum)  
**+** Arbitrary code execution: WebAssembly VM ([Asmble](https://github.com/cretz/asmble))  
**+** SDK: frontend (JavaScript) and backend (Rust)  

Features that have not been released yet:  
**–** Batch validation layer: tx history verification, VM state updates  
**–** Secure metadata storage: batch validation state, security deposits  
**–** Dispute resolution layer: verification game  
**–** Data availability layer: Swarm storage for tx history  

We tentatively plan to let developers ingest Ethereum transactions, as well as data stored in Swarm/IPFS into Fluence to create decentralized query interfaces. We also plan that other languages such as C++ or JavaScript will be supported for backend development at some point.
