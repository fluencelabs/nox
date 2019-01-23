## Requirements
- `npm`

## Installation
- `npm install js-fluence-client`

## Description
`js-fluence-client` uses to interact with the Fluence cluster (it consists of `workers`) in the Fluence network. It is a frontend facade for an application deployed in Fluence. A developer can write a `web3` decentralized application using Metamask and Ethereum network as a point of trusted agreement between developers and computing power.

## Usage
Import a dependency:
```
import * as fluence from "js-fluence-client";
```

To have a connection with the Fluence cluster we need to create a session between the browser and all nodes or one node in this cluster.
There is two way to establish a connection:
- Using host and port directly (for debug purposes) to connect with `worker`:
```
let workerSession = fluence.createDefaultSession("<host>", <port>);
```
- Using installed MetaMask or deployed local Ethereum node:
```
let appSession = 
let appSessionPromise = fluence.createAppSession("<contract-address>", "<app-id>").then((responseSession) => {
    appSession = responseSession;
});
```
Actual Fluence contract address on Kovan chain is: `0x45CC7B68406cCa5bc36B7b8cE6Ec537EDa67bC0B`.
appId - is an ID of application registered in the contract. For example we can use appId of already deployed LlamaDB application: `0x0000000000000000000000000000000000000000000000000000000000000002`.

`createAppSession` interact with Ethereum blockchain, so it will return Promise<AppSession>. Where `AppSession` is combining all sessions of nodes in a cluster and additional info about these nodes.
To get single session to one node:
```
let workerSession = appSession.workerSessions[<worker-idx>].session
```

Then we can `invoke` commands to workers and get responses.
As it was earlier we will use LlamaDB application as an example. Invoke simple SQL commands to the cluster:
```
let response = workerSession.invoke("CREATE TABLE test_table (id INT, text VARCHAR(128))")
```
The invocation will send a request and return submitted transaction, that we can ignore or use to return a result:
```
let resultPromise = response.result();
resultPromise.then((r) => console.log(r.asString()))
```
Communication between the cluster and the client is the exchange of arrays of bytes. Result returning as `hex`, to transform it into a string, we can use `asString` method. To return raw bytes, we can simply call method `r.hex()`. 
 
And to send raw bytes, instead of `invoke` possible to use `invokeRaw(raw-hex-request)`.

Based on that several methods we can build decentralized web applications.

In `Examples` can find simple decentralized applications.  

# Examples

Simple app to start with. Interacts with predeployed Fluence cluster on Kovan chain:
https://github.com/fluencelabs/frontend-example

App to write SQL requests from a browser (also gets info about Fluence nodes) written on TypeScript:
https://github.com/fluencelabs/fluence/tree/master/js-client/src/examples/fluence-sqldb
