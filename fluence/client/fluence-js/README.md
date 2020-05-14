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
let client1 = await Fluence.connect("12D3KooWBUJifCTgaxAUrcM9JysqCcS4CS8tiYH5hExbdWCAoNwb", "104.248.25.59", 9003, peerId1);
let client2 = await Fluence.connect("12D3KooWHk9BjDQBUqnavciRPhAYFvqKBe4ZiPPvde7vDaqgn5er", "104.248.25.59", 9002, peerId2);
```

Create a new unique service by the first client that will calculate the sum of two numbers.
```typescript
let serviceId = "sum-calculator-" + genUUID();

await client1.registerService(serviceId, async (req) => {   
    let message = {msgId: req.arguments.msgId, result: req.arguments.one + req.arguments.two};

    await client1.sendCall(req.reply_to, message);
});
```

Send a request by the second client and print a result. The predicate is required to match a request and a response by `msgId`.
```typescript
let msgId = "calculate-it-for-me" + genUUID();
let req = {one: 12, two: 23, msgId: msgId};

let predicate = (args: any) => args.msgId && args.msgId === msgId;

let response = await client2.sendServiceCallWaitResponse(serviceId, req, predicate);

let result = response.result;
console.log(`calculation result is: ${result}`);
```