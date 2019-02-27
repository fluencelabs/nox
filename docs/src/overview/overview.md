# Overview

The Fluence network consists of nodes performing computations in response to transactions sent by external clients. Algorithms specifying those computations are expressed in the WebAssembly bytecode; consequently, every node willing to participate in the network has to run a WebAssembly virtual machine.

Independent developers are expected to implement a backend package handling client transactions in a high-level language such as C/C++, Rust, or TypeScript, compile it into one or more We- bAssembly modules, and then deploy those modules to the Fluence network. The network then takes care of spinning up the nodes that run the deployed backend package, interacting with clients, and making sure that client transactions are processed correctly.

## Architecture

Two major layers exist in the Fluence network: the real-time processing layer and the batch valida- tion layer. The former is responsible for direct interaction with clients; the latter, for computation verification. In other words, real-time processing is the speed layer and batch validation is the security layer. The network also relies on Ethereum (as a secure metadata storage and dispute resolution layer) and Swarm (as a data availability layer) 

<div style="text-align:center">
<kbd>
<img src="../images/architecture_overview.png" width="666px"/>
</kbd>
<br><br><br>
</div>
