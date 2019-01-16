## Fluence Prerequisites:
- deployed public nodes with LlamaDB
- published info about nodes (hardcoded or dashboard)

## User prerequisites
- `npm`

# FrontEnd Developer Flow
- create directory `mkdir fluence-frontend`
- `cd fluence-frontend`
- `npm init -y` to silently init project
- `npm install --save js-fluence-client`
- to quick run in browser we can use webpack, so run `npm install --save-dev webpack-cli html-webpack-plugin` or if you expierenced in some other tools, feel free to use what is convenient for you
- create file `webpack.config.js` 
- add code in this file:
```
const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');

module.exports = {
    // use index.js as entrypoint
    entry: {
        app: ['./index.js']
    },
    mode: "development",
    // build all code in `bundle.js` in `bundle` directory
    output: {
        filename: 'bundle.js',
        path: path.resolve(__dirname, 'bundle')
    },
    plugins: [
        // create `index.html` with imported `bundle.js`
        new HtmlWebpackPlugin()
    ]
};
```
- add to `package.json` in the `scripts` part `"build": "webpack"` to build the project
- create file `index.js` and open it in any editor or IDE
- import fluence client `import * as fluence from "js-fluence-client"`
- to create session to interact with fluence cluster
- `const fluenceSession = fluence.createDefaultSession("<ip-to-node>", <port-to-node> todo: p2pPort+100, TODO add some tweaks in dashboard to find this info);`
- ip and port get from dashboard for public nodes with Llamadb
- let's try to use this session directly in browser
- add this to `index.js`: `window.fluenceSession = fluenceSession;`
- run `npm run build` in terminal
- open `bundle/index.html` from browser
- open console in browser (F12 in chrome on Ubuntu for example)
- write `let invocation = fluenceSession.invoke("do_query", "create table <some-unique-name> (id varchar(128))")`
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