# Fluence browser client
Browser client for the Fluence network based on the js-libp2p. It could exchange messages between relay peer, process AIR (Aquamarine Intermediate Representation) <link to air part for explanations> scripts, register local services and publish them in Fluence network

## How to install

With `npm` installed building could be done as follows:

```bash
npm install fluence
```

## Documentation

### Register local services
Firstly, let's register services, that could be called from AIR  scripts (later).

```typescript
// TODO somehow delete 'dist' directory from paths
import {Service} from "fluence/dist/service";
import {registerService} from "fluence/dist/globalState";

let service = new Service("custom-service-id")
service.registerFunction("custom-function-name", (args: any[]) => {
    console.log("custom-function-name called")
})
```

### Call a local function from an AIR script

A function could be called from AIR that a client will create and execute or if a client will receive a script from a relay peer. Let's just consider now just calling a function locally. 
```typescript
import Fluence from "fluence";
import {build} from "fluence/dist/particle";

// this is an address of a relay peer that a client will be connected with
let multiaddr = "/ip4/1.1.1.1/tcp/19001/wss/p2p/12D3KooWEXNUbCXooUwHrHBbrmjsrpHXoEphPwbjQXEGyzbqKnE9"

// second argument could be client's peerId. If it is empty, a new peerId will be generated
let client = await Fluence.connect(multiaddr);

// a script, that will call registered function
// call should be targeted with client's peer, name of a service and a function
let script = `(call ${client.selfPeerIdStr} ("custom-service-id" "custom-function-name") [])`

// build a particle, that combine script and data (that is empty for now)
let particle = await build(client.selfPeerId, script, new Map())

await client.executeParticle(particle)
// "custom-function-name called" should be printed in a console
```

### Pass arguments to a function
...

### Call a function from a remote client

Every peer in a Fluence network could have services, including clients and relay peers. Try to call a function that is registered on a relay that a client connected to. To do this, you just need to change the script.

```typescript
let script = `(call "relay-peer-id" ("custom-service-id" "custom-function-name") [])`
```

Aquamarine (a medium that could handle AIR) will see that call should be executed not on a client and give a signal to the client to send a particle to a relay.

But we cannot control relay peers if they don't belong to us. So, let's call a remote function from an another client. Create another client (in another tab, for example) and run a script that will describe whole path from one client to another.

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

Whole path described in a script. `identity` is only to indicate where to send a particle. 
After execution of particle starting a path will be: `-> "client2-relay-peer-id" -> "client1-relay-peer-id" -> "client1-peer-id" -> call a function`. And then, as a result, `custom-function-name called` message will be in a console. 

### Identity

Firstly, we don't want to generate new `peerId` on each client creation. Let's create a new `peerId` separately.

```typescript
let peerId = await Fluence.generatePeerId()
console.log(peerId.toB58String())

let client = await Fluence.connect(multiaddr, peerId);
``` 

You can store a peerId as a seed on a local machine or in some other safe place.
```typescript
import {peerIdToSeed, seedToPeerId} from "fluence/dist/seed";

let seed = peerIdToSeed(peerId)
// store it somewhere and than transform back to peerId
let peerId = seedToPeerId(peerId)
```

### How to control/use authentication & authorization from within JS SDK
- TBD

### Register wasms on a remote peers
...

### Add providers of services in a Fluence network (Kademlia)
...

## Examples 

- complex example with registering services in a network, using call results in AIR and calling multiple remote clients and peers 
- link to demo
