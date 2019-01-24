## IMPORTANT: Client and README are under heavy development and can be outdated. Ask a question in gitter or file an issue. 

## Fluence Javascript Client

**Fluence Javascript Client** allows you to interact with a real-time cluster from a browser or use it like `npm` module. 

## Motivation

Browser client simplifies interaction with the real-time cluster for a user. It has zero dependencies and can be added to other code.
Developers can deploy their backend code to Fluence network and built a web3 application based on **Fluence Client** to interact with this backend code.
It is written on TypeScript to reach more clean, understandable and typesafe code with good reusability.


## Documentation



### Installation

```
npm install fluence-js
``` 

### Before started 

You need to have a deployed real-time cluster. 
You can use [this part of project](https://github.com/fluencelabs/fluence/tree/master/statemachine) to deploy 4-nodes cluster with `increment` and `multiply` functions as an example.

**TODO:** HOWTO deploy custom code (link to some other readme)

### Init client and session

For interaction with the default `increment and multiply` cluster you can use `IncrementAndMultiply` class. 
If we have a local cluster, we can initialize client like this:

```typescript
let client = new IncrementAndMultiply("localhost", 46157);
```

Than we can use built-in functions:

```typescript
// mutate state but returns nothing
client.incrementCounter();

// get result after counter
let counter = await client.getCounter().result();
console.log(JSON.stringify(counter));

// get result of moltiplying
let res = await client.multiply(107, 124).result();

// should be 13268
console.log(JSON.stringify(res));
```

If you deployed your own code, you can use `CustomCommands`

```typescript
let client = new CustomCommands("localhost", 46157);

// will return some result based on logic of deployed code
let res = await client.submit("<some-custom-command>").result();
```

If you want to write your own library, you can use `IncrementAndMultiply.ts` as a basis:

```typescript
import {TendermintClient} from "../TendermintClient";
import {Engine} from "../Engine";
import {Signer} from "../Signer";
import {Client} from "../Client";
import {Session} from "../Session";

class IncrementAndMultiply {

    // a session is needed for ordered transactions, you can use multiple sessions if needed
    private session: Session;

    constructor(host: string, port: number) {
        // there is initializing RPC to tendermint cluster
        let tm = new TendermintClient(host, port);

        // creates engine that can start new sessions
        let engine = new Engine(tm);

        // default signing key for now
        // signing key can be generated or received after some authorize processes
        let signingKey = "TVAD4tNeMH2yJfkDZBSjrMJRbavmdc3/fGU2N2VAnxT3hAtSkX+Lrl4lN5OEsXjD7GGG7iEewSod472HudrkrA==";

        // creates signer that can sign messages
        let signer = new Signer(signingKey);

        // `client002` is a default client for now
        // creates client with id and signer
        let client = new Client("client002", signer);

        // generates the random session. If you want to generate session on your own - use createSession(client, "some-id")
        this.session = engine.genSession(client);
    }

    // uses the session to submit commands you want to
    async incrementCounter() {
        console.log("do increment");
        return this.session.invoke("inc");
    }

    async getCounter() {
        let res = await this.session.invoke("get");
        console.log(`get result is: ${JSON.stringify(res)}`);
        return res
    }

    async multiply(first: number, second: number) {
        let res = await this.session.invoke("multiplier.mul", [first.toString(), second.toString()]).result();
        console.log(`multiply result is: ${JSON.stringify(res)}`);
        return res;
    }
}
```

You can find other examples in the [examples](https://github.com/fluencelabs/fluence/tree/master/js-client/src/examples) folder.

