# Fluence browser client
Browser client for the Fluence network based on the js-libp2p. It could exchange messages between relay peer, process AIR (Aquamarine Intermediate Representation) <link to air part for explanations> scripts, register local services and publish them in the Fluence network.

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

// the second argument could be the client's peer id. If it is empty, a new peer id will be generated
let client = await Fluence.connect(multiaddr);

// a script, that will call registered function
// a call should be targeted with the client's peer, name of a service and a function
// %init_peer_id% - it is a predefined variable with peer id of a caller 
let script = `(call %init_peer_id% ("custom-service-id" "custom-function-name") [])`

// build a particle, that combines script and data (that is empty for now)
let particle = await build(client.selfPeerId, script, new Map())

await client.executeParticle(particle)
// "custom-function-name called" should be printed in a console
```

### Pass arguments to a function
It is possible to pass arguments to a function and use the result in AIR scripts:
```typescript
let script = `(call %init_service_id% ("custom-service-id" "custom-function-name") ["arg1" "arg2" "arg3"] result)`
```
The result could be used strictly in the AIR script. Arguments could be strings or variables with custom types. You can use it just like that:
```typescript
service.registerFunction("custom-function-name", (args: any[]) => {
    console.log("arg1 = " + args[0])
    console.log("arg2 = " + args[1])
    console.log("arg3 = " + args[2])
    let result = args.join(",")
    return result 
})
``` 

About variables in AIR in the next part below.

### Particle
Particle is a combination of AIR script, data for script execution and utility information about security, time-to-live, etc. We only used scripts with hardcoded string arguments, but it could be more flexible with custom variables. Variables are specified without quotes. Client should add these variables to data or AIR script will be failed on execution. 
```typescript
let script = `(call %init_service_id% (service_id function_name) [arg1 arg2] result)`

let data = new Map()
data.set("service_id", "some-service")
data.set("function_id", "some-function")
data.set("arg1", {customObject: 12, withCustomTypes: {a: 1, b: "string"}})
data.set("arg2", "arg2")

let particle = await build(client.selfPeerId, script, data)
```

How to use variables in AIR script in detail here (LINK).

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

Firstly, we don't want to generate a new `peerId` on each client creation. Let's create a new `peerId` separately.

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

Firstly, the client should create a blueprint (a combination of modules) to create services from it. It will return a blueprint id that client can use to create a service. Then services could be called from AIR script.

```typescript
// modules could be linked to each other. If so, dependent modules should be specified after dependencies.
let blueprintId = await client.addBlueprint("blueprint name", ["wasm1 name", "wasm2 name"], remotePeerId)
let serviceId = await client.createService(blueprintId, remotePeerId)
```

### Add providers of services in a Fluence network (Kademlia)
TBD

## Examples 
 
[Chat example](https://github.com/fluencelabs/aqua-demo/tree/master/demo)
