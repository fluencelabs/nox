## What is it?

This package is an example of how is it possible to integrate the fluence cluster with a data source.

[streamr](https://www.streamr.com/) has been taken as a data source.

Take a look at how we can take data from a source and insert it into LlamaDB, that covered under the `fluence` cluster and get some aggregated information.

## What is `fluence`

Take a look at [the main repo](https://github.com/fluencelabs/fluence) for necessary information.

[Here](https://github.com/fluencelabs/workshop-2018-oct) you can find how to create a cluster and do the first steps of how to use `fluence`.  

## Step by step

To run code from this repo:
1. Clone repo to any directory
2. `cd` to this directory
3. Run `npm install` command
4. Run `npm run build` command
5. Open `./bundle/index.html` in your browser
6. Open terminal in a browser
7. Call `getData()` to open stream and insert rows of data from it
8. Call `getCount()` to count number of rows
9. Call `getMax(parameter)` to get maximum of a parameter, where parameter could be one of `pm25, pm10, no2, o3`

To write code from scratch: 

1. Initialize project with `npm init`
2. Install `fluence` and `streamr` clients with `npm install --save js-fluence-client streamr-client`
3. Create `index.js` file and open it with any editor
4. Import clients and initialize them
```javascript
import StreamrClient from 'streamr-client'
import * as fluence from "js-fluence-client"

// with default options it will be connected to `wss://www.streamr.com/api/v1/ws`
const streamrClient = new StreamrClient();

// creates session with predefined credentials, cluster should be started already
const fluenceSession = fluence.createDefaultSession("localhost", 29057);
```
5. Create table in LlamaDB in the `fluence` cluster
```javascript
// it is a simplified query, there is more parameters, full query is in the repo
const createTableQuery = "CREATE TABLE polution_uusimaa(id varchar(128), location varchar(128), parameter varchar(128), value double, unit varchar(128))";

fluenceSession.invoke(createTableQuery)
    .result() // to return promise and wait for result we need to call `result()` function
    .then((r) => console.log(r.asString())); // `asString()` decodes bytes format to string
```
6. Explain of how we will inserts new data to LlamaDB
```javascript
function genInsertQuery(data) {
    const query = `INSERT INTO polution_uusimaa VALUES ('${data.id}', '${data.location}', '${data.parameter}', ${data.value}, '${JSON.stringify(data.unit)}')`;
    console.log("Query: " + query);
    return query;
}
```
7. Get data
```javascript
streamrClient.subscribe(
        {
            stream: 'dVoD8tzLR6KLJ-z_Pz8pMQ', // it is an id of stream of pollution data in Uusimaa, Finland
            resend_last: 100 // will send last 100 rows
        },
        (message, metadata) => {
            const query = insertQuery(message); // generates query
            fluenceSession.invoke(query); // and inserts it in into LlamaDB
        }
    )
```
8. As an example, create functions that will count our data and get maximum:
```javascript
function getCount() {
    const query = "select count(*) from polution_uusimaa";
    fluenceSession.invoke(query).result().then((r) => {
        console.log("Data count: " + r.asString().split("\n")[1])
    })
}

/**
 * Get maximum value of parameter
 *
 * @param parameter [pm25, pm10, no2, o3]
 */
function getMax(parameter) {
    const query = `select max(value) from polution_uusimaa where parameter = '${parameter}'`;
    fluenceSession.invoke(query).result().then((r) => {
        console.log(`Maximum of ${parameter}: ${r.asString().split("\n")[1]}`);
    })
}
```
