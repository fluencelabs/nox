# Introduction

## What is Fluence?

Fluence is an efficient trustless computation platform that allows to achieve few seconds request processing latency and cost efficiency similar to traditional cloud computing. To run computations, Fluence uses a WebAssembly virtual machine, which allows to deploy into the decentralized environment applications written in multiple programming languages.


## What can be built with Fluence?

Fluence can be used as a general purpose backend engine for decentralized applications. Because of its cost efficiency, developers generally do not have to worry much about low-level code optimization. Existing software packages can be ported to Fluence as is, once they are compiled into WebAssembly.

#### Decentralized databases

As an [example](https://github.com/fluencelabs/tutorials/tree/master/llamadb), we have ported an existing toy SQL database [LlamaDB](https://github.com/nukep/llamadb) to run in the decentralized environment by making just a few modifications and compiling it into WebAssembly.

We expect it should not be extremely difficult to port an existing database such as SQLite or RocksDB to Fluence. Deployed, this database could serve frontends of user-facing decentralized applications. 

This, coupled with decentralized storages such as IPFS or Swarm could enable fully decentralized applications. Such applications would use a decentralized storage to store static data, and a decentralized database running on top of Fluence – to serve dynamic client requests.

#### Gambling applications

Simple dice games or roulette can be ported to Fluence fairly easily. However, at the current moment Fluence is not able to run by itself [imperfect information games](https://en.wikipedia.org/wiki/Perfect_information) such as Texas Holdem poker or Guess the Number. The reason is that full game state can be read by nodes running the backend, and Fluence does not have privacy-preserving computations built in into its SDK.

We expect, however, that approaches such as [decentralized card deck shuffling](https://ethereum.stackexchange.com/questions/376/what-are-effective-and-secure-ways-of-shuffling-a-deck-of-cards-in-a-contract/758) can be employed to port certain imperfect information games to the Fluence network. In the meanwhile we have prepared a _[broken implementation](https://github.com/fluencelabs/tutorials/tree/master/guessing-game)_ of Guess the Number game ;)

#### Games

Perfect information games can be easily built on top of Fluence. Think of chess, Go, rock–paper–scissors, games similar to Dungeons and Dragons, or roll-and-move board games. We have prepared a [tic-tac-toe](https://github.com/fluencelabs/tutorials/tree/master/tic-tac-toe) example to play with. Collectible decentralized applications can be launched on Fluence as well.


## How does Fluence work?

In order to reach low latency and high throughput, Fluence splits network nodes into two layers: the _real-time processing layer_ and the _batch validation layer_. 

The real-time processing layer is able to promptly serve client requests, but provides only moderate security guarantees that returned responses are correct. Later, the batch validation layer additionally verifies returned responses, and if it is found that some of the responses were incorrect, offending real-time nodes lose their deposits.

In this section we provide brief Fluence overview and discuss reasons to have delayed verification. We also consider Fluence basic incentive model.


