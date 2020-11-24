# Fluence JS SDK
Fluence JS SDK is a bridge to Fluence Network. It provides you a local Fluence Peer, powering you to develop your application in p2p fashion. 

SDK gives you the following: 
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

// this is an address of a relay peer that a client will be connected with
let multiaddr = "/ip4/1.1.1.1/tcp/19001/wss/p2p/12D3KooWEXNUbCXooUwHrHBbrmjsrpHXoEphPwbjQXEGyzbqKnE9"

// the second argument could be the client's peer id. If it is empty, a new peer id will be generated
let client = await Fluence.connect(multiaddr);

// a script, that will call registered function
// call is an instruction that takes the following parameters
// (call <Peer location> <Function location> <[argument list]> <optional output>)
// %init_peer_id% - it is a predefined variable with peer id of a caller 
let script = `(call %init_peer_id% ("console" "log") ["hello"])`

// Wrap script into particle, so it can be executed by local WASM runtime
let particle = await build(client.selfPeerId, script, new Map())

await client.executeParticle(particle)
// "[hello]" should be printed in a console
```

### Pass values as function arguments
We've seen how to pass literal arguments (i.e. values in quotes). Using literals can be tedious if you need to repeat values, or wish to keep script short and readable. To avoid that, you can use variables that refer to particle data.

```typescript
let script = `(call %init_peer_id% ("console" "log") [msg])`;
let particle = await build(client.selfPeerId, script, new Map("msg" -> "hello"));
await client.executeParticle(particle)
```

To learn more about writing AIR scripts, refer to [doc on AIR](https://fluence-labs.readme.io/docs/air-choreography-scripts).

### Call a function from a remote client

Every peer in a Fluence network could have services, including clients and relay peers. Let's call a function that is registered on a relay that a client connected to. To do this, you just need to change the script.

```typescript
let script = `(call "relay-peer-id" ("custom-service-id" "custom-function-name") [])`
```

Aquamarine (a medium that could handle AIR) will see that call should be executed not on a client and give a signal to the client to send a particle to a relay.

But we cannot control relay peers if they don't belong to us. So, let's call a remote function from another client. Create another client (in another tab, for example) and run a script that will describe the whole path from one client to another.

```typescript
// second client that connected to another relay peer
let multiaddr2 = "/ip4/2.2.2.2/tcp/19001/wss/p2p/12D3KooWEXNUbCXooUwHrHBbrmjsrpHXoEphPwbjQXEGyzbqKnE9"
let client2 = await Fluence.connect(multiaddr2);

// here we will use 'seq' command to combine multiple calls
let script = `
(seq
    (call "client2-relay-peer-id" ("identity" "") [])
    (seq
        (call "client1-relay-peer-id" ("identity" "") [])  
        (seq
            (call "client1-peer-id" ("custom-service-id" "custom-function-name") [])
        )
    )
)
`
```

The whole path described in a script. `identity` is only to indicate where to send a particle. 
After execution of particle starting a path will be: `-> "client2-relay-peer-id" -> "client1-relay-peer-id" -> "client1-peer-id" -> call a function`. And then, as a result, `custom-function-name called` message will be in a console. 

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
