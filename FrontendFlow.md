## Fluence Prerequisites:
- deployed public nodes with LlamaDB
- published info about nodes (hardcoded or dashboard)
- js client

## User prerequisites
- `npm`

# FrontEnd Developer Flow
- for example we can use template repository to learn how to use Fluence SDK
- `git clone git@github.com:fluencelabs/frontend-template.git`
- Install fluence SDK: `npm install --save js-fluence-client@0.0.34`
- import fluence client `import * as fluence from "js-fluence-client"` in `index.js`
- to create session to interact with fluence cluster we have two ways:
  - To create session to one node we can use IP and port from contract: `const fluenceSession = fluence.createDefaultSession("<ip>", <port>);`
  - We can create multiple sessions to all cluster nodes if we know appId of application: `fluence.createAppSession(contractAddress, appId)`. Note that it will return promise and Metamask or local Ethereum node is needed.
- ip and port get from dashboard for public nodes with Llamadb
- let's try to use this session directly in browser
- add this to `index.js`: `window.fluenceSession = fluenceSession;`
- run `npm run build` in terminal
- open `bundle/index.html` from browser
- open console in browser (F12 in chrome on Ubuntu for example)
- write `let invocation = fluenceSession.invoke("create table <some-unique-name> (id varchar(128))")`
- here we send request to the Fluence cluster to create table with some unique name for your own with one field `id`
- response from the cluster can be received eventually, so to get response we can call `result()` method that will return `Promise<String>` with array in hex representation
- to log response, write `invocation.result().then((r) => console.log(r.asString()));`. `asString()` converts hex to string, because we know, that LlamaDB returns strings on request
- the result can be `table created` or `[Error] Table <some-unique-name> already exists` depends on table name
- after this we can insert some values `fluenceSession.invoke("do_query", "insert into <some-unique-name> values ('123')").result().then((r) => console.log(r.asString()));`
- and select them `fluenceSession.invoke("do_query", "select * from <some-unique-name>").result().then((r) => console.log(r.asString()));`
- based on these requests we can write some simple app, storing values in LlamaDB
- interact with results on web page, add some buttons and text fields

# Examples

Simple `todo-list` based on Fluence:
https://github.com/fluencelabs/fluence/tree/master/js-client/src/examples/todo-list

Naive integration with streamr:
https://github.com/fluencelabs/fluence/tree/master/js-client/src/examples/fluence-streamr

App to write SQL requests from a browser (also gets info about Fluence nodes) written on TypeScript:
https://github.com/fluencelabs/fluence/tree/master/js-client/src/examples/fluence-sqldb

A step-by-step guide to deploying nodes locally, publish a rust-based app to Fluence contract (to run cluster) and write a simple app on JavaScript: 
https://github.com/fluencelabs/workshop-2018-dec