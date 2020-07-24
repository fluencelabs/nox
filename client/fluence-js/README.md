# Fluence browser client
Browser client for the Fluence network based on the js-libp2p.

## How to build

With `npm` installed building could be done as follows:

```bash
npm install fluence
```

## Example 

Shows how to register and call new service in Fluence network.


Generate new peer ids for clients.
```typescript
let peerId1 = await Fluence.generatePeerId();
let peerId2 = await Fluence.generatePeerId();
```

Establish connections to predefined nodes.

```typescript
let client1 = await Fluence.connect("/dns4/134.209.186.43/tcp/9003/ws/p2p/12D3KooWBUJifCTgaxAUrcM9JysqCcS4CS8tiYH5hExbdWCAoNwb", peerId1);
let client2 = await Fluence.connect("/ip4/134.209.186.43/tcp/9002/ws/p2p/12D3KooWHk9BjDQBUqnavciRPhAYFvqKBe4ZiPPvde7vDaqgn5er", peerId2);
```

## Become a provider

Create a new unique name to provide by the first client that will calculate the sum of two numbers.
```typescript
let name = "sum-calculator-" + genUUID();

await cl1.provideName(name, async (req) => {   
    let message = {msgId: req.arguments.msgId, result: req.arguments.one + req.arguments.two};

    await cl1.sendCall(req.reply_to, message);
});
```

## Become a provider

To declare that you're available on some token (unique name), you can become a provider of that token. For example, below you become a provider of a name `sum-calculator-1234...` so you can share that name with other people, and they can call you by that.

```typescript
let req = {one: 12, two: 23, msgId: msgId};

let response = await client2.callProvider(name, req);

let result = response.result;
console.log(`calculation result is: ${result}`);
```



## Register Service
Will register service that will combine multiple modules around one serviceId
```
let serviceId = await cl2.createService(peerAddr, ["ipfs_node.wasm", "curl.wasm"]);
console.log(serviceId);
```

## Call Service

```
// peerAddr – address of the node that runs this service
// "get_address" – function to call
// { some_arg: "1" } – arguments passed to the function
// "ipfs_node.wasm" – name of the module to find function in

let resp = await cl2.callService(peerAddr, serviceId, "ipfs_node.wasm", {some_arg: "1"}, "get_address")
console.log(resp)
```

## Discover Services

```
// get available modules on node (to get info about connected node should use the method without arguments)
let modules = cl1.getAvailableModules(peerAddr);

// get interfaces of existing services
let interfaces = await cl2.getActiveInterfaces(peerAddr);
```
