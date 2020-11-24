# Fluence JS SDK
Fluence JS SDK is a bridge to Fluence Network. It provides you a local Fluence Peer, powering you to develop your application in p2p fashion. 

SDK gives you the following powers: 
- Means to create and manage identity of your local peer and applications
- Ability to execute AIR scripts on local WASM runtime
- Define behaviour of local function calls (i.e., when script calls function on your local peer)
- Automatically forward AIR script execution to remote peers, according to script-defined topology

## How to install

If you have `npm`, you can add fluence to your project by executing

```bash
npm install fluence
```

## Documentation

### Define local services
First, let's register a local service `console` and define its behaviour by registering function `log`, as in `console.log`. We'll show how to call it in the next step.

```typescript
// TODO somehow delete 'dist' directory from paths
import {Service} from "fluence/dist/service";
import {registerService} from "fluence/dist/globalState";

let service = new Service("console")
service.registerFunction("log", (args: any[]) => {
    console.log(`log: ${args}`)
})
```

### Call function on local service

As AIR scripts describe topology of execution functions on peers, we can write a script to call a function on our local `console` service.

Script could be as follows
```clojure
(call %init_peer_id% ("console" "log") ["hello" "from" "WASM"])
```

`init_peer_id` refers to a peer that initiated script execution. In that case it is us, so call of `console.log` will call previously defined function `call` on service `console`.

Here's how this can be expressed in terms of Fluence JS SDK.
```typescript
import Fluence from "fluence";
import {build} from "fluence/dist/particle";

/* 

TODO: How to avoid connecting to remote relay here? execute particle could work locally, and throw error if connection is required.

// this is an address of a relay peer that a client will be connected with
let multiaddr = "/ip4/1.1.1.1/tcp/19001/wss/p2p/12D3KooWEXNUbCXooUwHrHBbrmjsrpHXoEphPwbjQXEGyzbqKnE9"

// the second argument could be the client's peer id. If it is empty, a new peer id will be generated
let client = await Fluence.connect(multiaddr);
*/

// call is an instruction that takes the following parameters
// (call <Peer location> <Function location> <[argument list]> <optional output>)
let script = `(call %init_peer_id% ("console" "log") ["hello"])`

// Wrap script into particle, so it can be executed by local WASM runtime
let particle = await build(client.selfPeerId, script, new Map())

await client.executeParticle(particle)
// "[hello]" should be printed in a console
```

### Using variables as function arguments
We've seen how to pass literal arguments (i.e. values in quotes). Using literals can be tedious if you need to repeat values, or wish to keep script short and readable. To avoid that, you can use variables that refer to particle data.

```typescript
let script = `(call %init_peer_id% ("console" "log") [msg])`;
let particle = await build(client.selfPeerId, script, new Map("msg" -> "hello"));
await client.executeParticle(particle)
```

To learn more about writing AIR scripts, refer to [doc on AIR](https://fluence-labs.readme.io/docs/air-choreography-scripts).

### Showcase: relaying & remote execution
Fluence network is made of peers of various execution power, availability guarantees and most importantly – various connectivity. To allow peers from non-public networks to communicate, Fluence employs *relay* mechanics. Currently, any Fluence Node can be used as a relay.

To learn more about relaying, refer to the [doc](https://fluence-labs.readme.io/docs/on-relays).

For now, we'll use relay to connect two browser peers. You can emulate two peers by opening two browser tabs, for example. I'll assume that you have done so, and their peer ids are `123DPeerIdA` and `123DPeerIdB`.

We'll use the following relays:
- `/dns4/stage.fluence.dev/tcp/19001/wss/p2p/12D3KooWEXNUbCXooUwHrHBbrmjsrpHXoEphPwbjQXEGyzbqKnE9`
- `/dns4/stage.fluence.dev/tcp/19002/wss/p2p/12D3KooWHk9BjDQBUqnavciRPhAYFvqKBe4ZiPPvde7vDaqgn5er`

On a first browser, connect to first relay, and register service with a single function as follows.
```typescript
// TODO somehow delete 'dist' directory from paths
import {Service} from "fluence/dist/service";
import {registerService} from "fluence/dist/globalState";

let service = new Service("console")
service.registerFunction("log", (args: any[]) => {
    console.log(`log: ${args}`)
})

let multiaddr = "/dns4/stage.fluence.dev/tcp/19001/wss/p2p/12D3KooWEXNUbCXooUwHrHBbrmjsrpHXoEphPwbjQXEGyzbqKnE9"
let client = await Fluence.connect(multiaddr);
console.log(`First PeerId: ${client.selfPeerId}`);
```

On a second browser, connect to second relay, and call remote `console.log` as follows.
```typescript
let multiaddr = "/dns4/stage.fluence.dev/tcp/19002/wss/p2p/12D3KooWHk9BjDQBUqnavciRPhAYFvqKBe4ZiPPvde7vDaqgn5er"
let client = await Fluence.connect(multiaddr);
console.log(`Second PeerId: ${client.selfPeerId}`);

let script = `
    (seq
        (call second-relay ("op" "identity") [])
        (seq
            (call first-relay ("op" "identity") [])
            (call first-peer ("console" "log") [msg])
        )
    )
`;
let particle = await build(
    client.selfPeerId, script, 
    new Map(
        "first-peer" -> "123DPeerIdA" // <== Do not forget to change 123DPeerIdA to actual peer id
        "second-relay" -> "12D3KooWHk9BjDQBUqnavciRPhAYFvqKBe4ZiPPvde7vDaqgn5er",
        "first-relay" -> "12D3KooWEXNUbCXooUwHrHBbrmjsrpHXoEphPwbjQXEGyzbqKnE9",
        "msg" -> "hello"
    )
);
await client.executeParticle(particle)
```

After that, you should see message `log: [hello]` in console of the first browser.

To learn more about AIR scripts, refer to [doc](https://fluence-labs.readme.io/docs/air-scripts)

### Identity

First, we don't want to generate a new `peerId` on each client creation. Let's create a new `peerId` separately.

```typescript
let peerId = await Fluence.generatePeerId()
console.log(peerId.toB58String())

let client = await Fluence.connect(multiaddr, peerId);
``` 

You can store a peerId as a seed on a local machine or in some other safe place.
```typescript
import {peerIdToSeed, seedToPeerId} from "fluence/dist/seed";

let seed = peerIdToSeed(peerId)
// store it somewhere and then transform back to peerId
let peerId = seedToPeerId(peerId)
```

### How to control/use authentication & authorization from within JS SDK
- TBD

### Upload wasm modules, add blueprints and create services on a remote peers   
The client can upload wasm modules and other peers could use these modules to create their own services. These modules could be presented in a base64 encoding.
```typescript
let client = await Fluence.connect(multiaddr);

// remote peer could be 'undefined' for relay or peerId in base58 for other remote peers in Fluence network
let remotePeerId = undefined

await client.addModule("wasm1 name", WASM1_BS64, remotePeerId);
await client.addModule("wasm2 name", WASM2_BS64, remotePeerId);
```

First, the client should create a blueprint (a combination of modules) to create services from it. It will return a blueprint id that client can use to create a service. Then services could be called from AIR script.

```typescript
// modules could be linked to each other. If so, dependent modules should be specified after dependencies.
let blueprintId = await client.addBlueprint("blueprint name", ["wasm1 name", "wasm2 name"], remotePeerId)
let serviceId = await client.createService(blueprintId, remotePeerId)
```

### Add providers of services in a Fluence network (Kademlia)
TBD

## Examples 
 
[Chat example](https://github.com/fluencelabs/aqua-demo/tree/master/demo)
