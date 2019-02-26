## Requirements
- `npm`

## Installation
- `npm install fluence`

## Description
`js-fluence-client` is a library written in TypeScript that allows you to interact with an app (usually a backend) deployed to a Fluence cluster in the Fluence network, from a web browser. Using this library together with Metamask and Ethereum network, you can develop a `web3` decentralized application.

## Usage
Import a dependency:
```js
import * as fluence from "fluence";
```

To connect to the Fluence cluster we need to create a session between the browser and all nodes or one node in this cluster.
There is two ways of establishing a connection:
1. Using host and port directly (for debug purposes) to connect with a single node of the cluster, or simply `worker`:
```js
let workerSession = fluence.directConnect("<host>", <port>);
```
2. Using installed MetaMask or a deployed local Ethereum node:
```js
let appSession;
let appSessionPromise = fluence.connect("<contract-address>", "<app-id>", "<ethereum-url-optional>").then((responseSession) => {
    appSession = responseSession;
});
```
You can find Fluence contract on Rinkeby testnet deployed at address: `0x074a79f29c613f4f7035cec582d0f7e4d3cda2e7`.
The second argument, `appId` - is an ID of application registered in the contract. For example, there is an existing [LlamaDB](https://github.com/fluencelabs/llamadb) application with `appId`: `0x0000000000000000000000000000000000000000000000000000000000000002`.

`connect` asynchronously interacts with an Ethereum blockchain, so it will return a `Promise<AppSession>`. `AppSession` is a structure that combines all sessions to nodes in a cluster and keeps some metadata of these nodes.

Then we can use `invoke` to send commands to workers and get responses.
We'll go with [LlamaDB](https://github.com/fluencelabs/llamadb) application as an example. Send simple SQL commands to the cluster:
```js
let response = appSession.invoke("CREATE TABLE test_table (id INT, text VARCHAR(128))")
```
It will send a request and return the submitted transaction. Retrieving result requires calling `result()` method, that's because sending a transaction via `invoke` doesn't return result back, it just changes cluster state. `result()` method explicitly reads result of the transaction from the cluster state, and returns it as a `Promise<Result>`:
```js
let resultPromise = response.result();
resultPromise.then((r) => console.log(r.asString()))
```
Client and cluster are communicating by exchanging raw bytes. You can use `asString()` to convert bytes to UTF-8 string or `hex()` to get hex representation of the data.

In case you need fine-grained control, you can use `workerSession` directly:
```js
let workerSession = appSession.workerSessions[<worker-idx>].session
// send tx from string
let resultPromise = workerSession.invoke("SELECT ...");

// send tx from raw bytes
let resultPromise = workerSession.invokeRaw(<raw-bytes-in-hex>);
```

Given these simple methods, you can build a working decentralized web application!

You can find examples of such applications below.

# Example apps

Simple app to start with or use as a template for your app. Interacts with predeployed Llamadb on top of Fluence cluster connected to Kovan testnet:
https://github.com/fluencelabs/frontend-example

App that allows you to send SQL requests from a browser and shows status of the cluster's workers. Written in TypeScript:
https://github.com/fluencelabs/fluence/tree/master/js-client/src/examples/fluence-sqldb

# Acknowledgments
Thanks to @nukep for his Llamadb (https://github.com/nukep/llamadb) that we used in this example!