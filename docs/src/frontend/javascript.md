# Simple to use
Fluence client is designed to be the simplest way possible to make requests to the Fluence network. You don't need anything to know about the internals of Fluence, just use it like other common libraries. 

# Installation
`npm install --save fluence`

# Usage

We need to know three constants to work with Fluence:
```javascript
// address to Fluence contract in Ethereum blockchain. 
let contractAddress = "0xeFF91455de6D4CF57C141bD8bF819E5f873c1A01";

// set ethUrl to `undefined` to use MetaMask instead of Ethereum node
let ethUrl = "http://rinkeby.fluence.one:8545/";

// application to interact with that stored in Fluence contract
let appId = "43";
```

Let's create a session with Fluence and start to send requests. 

```javascript
import * as fluence from "fluence";

let session;

fluence.connect(contractAddress, appId, ethUrl).then((s) => {
    console.log("Session created");
	session = s;
});
```

Need to remember that all interactions are asynchronous, so we always work with promises.

Send request to backend:
```javascript
let request = s.request("Hello, Fluence!")
```

Sending a request and getting a response is different operations, because of the decentralized architecture of Fluence network. Call another method to get a response:
```javascript
request.result()
    .then((r) => console.log(r.asString()))
```

`asString()` method was called because in the raw form a response is bytes in hex representation.

That's all! Based on this API a developer can build client-server application any level of complexity.

# Tips and Tricks

### Sign all requests

You can add private key as a fourth argument in `connect` method if it is needed to check the correctness of signature on the side of a backend (authorization).
```javascript
fluence.connect(contractAddress, appId, ethUrl, privateKey)
```

More about authentication on a backend side: [Backend Guide: Best Practices](../backend/best_practices.md)

### Use MetaMask

Just left `ethUrl` argument as `undefined`
```javascript
fluence.connect(contractAddress, appId, undefined)
```

### Direct connection for local testing

Use `directConnect` to connect to local hosted node:
```javascript
let appId = 41 // it is not important on local nodes, could be random number
fluence.directConnect("localhost", 30000, appId);
```

For more details and how to debug the code: []
