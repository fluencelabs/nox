[![Build Status](https://travis-ci.org/fluencelabs/fluence.svg?branch=master)](https://travis-ci.org/fluencelabs/fluence) 
[![Join The Discord Chat](https://img.shields.io/discord/308323056592486420.svg)](https://discord.gg/9x5XCNd)  

## What is Fluence?

Fluence is an efficient trustless computation platform that allows to achieve few seconds request processing latency and cost efficiency similar to traditional cloud computing. To run computations, Fluence uses a WebAssembly virtual machine, which allows to deploy into the decentralized environment applications written in multiple programming languages.

## What can be built with Fluence?

Fluence can be used as a general purpose backend engine for decentralized applications. Because of its cost efficiency, developers generally do not have to worry much about low-level code optimization. Existing software packages can be ported to Fluence as is, once they are compiled into WebAssembly.

As a showcase, we have prepared few example applications that can be launched on the Fluence network:

- [Hello world](https://github.com/fluencelabs/tutorials/tree/master/hello-world/app-sdk-rust-2018) is a basic demo application demonstrating Fluence SDK usage
- [Llamadb](https://github.com/fluencelabs/tutorials/tree/master/llamadb) is a port of an existing in-memory SQL database
- [Tic-tac-toe](https://github.com/fluencelabs/tutorials/tree/master/tic-tac-toe) is a basic [perfect information](https://en.wikipedia.org/wiki/Perfect_information) multi-player game
- [Dice game](https://github.com/fluencelabs/tutorials/tree/master/dice-game) demonstrates how an RNG can be used to implement a simple gambling application
- [Guess the Number](https://github.com/fluencelabs/tutorials/tree/master/guessing-game) is an invalid implementation of an imperfect information game (which cannot be launched on Fluence at the current moment)
- [Streamr analytics](https://github.com/fluencelabs/tutorials/tree/master/streamr) is a simple analytics built on top of a [blockchain-based data marketplace](https://www.streamr.com/)

## How does Fluence work?

In order to reach low latency and high throughput, Fluence splits network nodes into two layers: the _real-time processing layer_ and the _batch validation layer_. 

The real-time processing layer is responsible for direct interaction with clients; the batch validation layer – for computation verification. In other words, real-time processing is the speed layer and batch validation is the security layer.

The real-time processing layer is able to promptly serve client requests, but provides only moderate security guarantees that returned responses are correct. Later, the batch validation layer additionally verifies returned responses, and if it is found that some of the responses were incorrect, offending real-time nodes lose their deposits. 

The network also relies on Ethereum (as a secure metadata storage and dispute resolution layer) and Swarm (as a data availability layer).

<img src="docs/src/images/architecture_overview.png" width="666"/>

Check out the [Fluence book](https://fluence.network/docs/book/introduction/overview.html) to learn more about the general Fluence architecture.

## Project status
The project is undergoing a heavy development at the moment.  
Check out the [issue tracker](https://github.com/fluencelabs/fluence/issues) to learn more about the current progress.

Features that were already rolled out:  
**+** Real-time processing layer: real-time clusters with built-in BFT consensus (Tendermint)  
**+** Secure metadata storage: real-time clusters state (Ethereum)  
**+** Arbitrary code execution: WebAssembly VM ([Asmble](https://github.com/cretz/asmble))  
**+** SDK: frontend (JavaScript) and backend (Rust)  

Features that have not been released yet:  
**–** Batch validation layer: tx history verification  
**–** Secure metadata storage: batch validation state, security deposits  
**–** Dispute resolution layer: verification game  
**–** Data availability layer: Swarm storage for tx history  

While we are working hard to make Fluence bulletproof secure, you can already build and deploy applications to the Fluence devnet! Take a look at the [quickstart](https://fluence.network/docs/book/quickstart/index.html) to learn how to build and decentralize a simple Hello World app, and at the [dice game tutorial](https://github.com/fluencelabs/tutorials/tree/master/dice-game) to learn how to build more involved apps.

## Resources

- [The Fluence book](https://fluence.network/docs/book/)
- [Key examples and tutorials](https://github.com/fluencelabs/tutorials)

## Contributing
You are welcome to contribute. At the current moment we don't have detailed instructions on how to join development or which code guidelines to follow. However, you can expect more info to appear soon enough. In the meanwhile, check out the [basic contributing rules](./CONTRIBUTING.md).

## License
[Apache 2.0](./LICENSE.md)
