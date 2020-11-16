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

### Dev: Update Aquamarine in repo

```
./build-aqua.sh path-to-aquamarine-dir
```
