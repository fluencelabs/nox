import StreamrClient from 'streamr-client'
import * as fluence from "js-fluence-client"

const streamrClient = new StreamrClient();
const fluenceSession = fluence.createDefaultSession("localhost", 29057);

const createTableQuery = "CREATE TABLE polution_uusimaa(id varchar(128), location varchar(128), parameter varchar(128), " +
    "value double, unit varchar(128), country varchar(128), city varchar(128), latitude double, " +
    "longitude double, local varchar(128), utc varchar(128))";

const deleteQuery = "DELETE FROM polution_uusimaa";

fluenceSession.invoke(deleteQuery).result().then((r) => console.log(r.asString()));

fluenceSession.invoke(createTableQuery)
    .result() // to return promise and wait for result we need to call `result()` function
    .then((r) => console.log(r.asString())); // `asString()` decodes bytes format to string

function insertQuery(data) {
    const query = `INSERT INTO polution_uusimaa VALUES ('${data.id}', '${data.location}', '${data.parameter}', ${data.value}, ` +
        `'${JSON.stringify(data.unit)}', '${data.country}', '${data.city}', ${data.latitude}, ${data.longitude}, '${data.local}', '${data.utc}')`;
    console.log("Query: " + query);
    return query;
}

function getData() {
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
}

function getCount() {
    const query = "SELECT COUNT(*) FROM polution_uusimaa";
    fluenceSession.invoke(query).result().then((r) => {
        console.log("Data count: " + r.asString().split("\n")[1])
    })
}

const parameters = ["pm25", "pm10", "no2", "o3"];

/**
 * Get maximum value of parameter
 *
 * @param parameter [pm25, pm10, no2, o3]
 */
function getMax(parameter) {
    if (!parameters.includes(parameter)) throw new Error(`No such parameter ${parameter}. Use only one of: ${parameters}`);
    const query = `SELECT MAX(value) FROM polution_uusimaa WHERE parameter = '${parameter}'`;
    fluenceSession.invoke(query).result().then((r) => {
        console.log(`Maximum of ${parameter}: ${r.asString().split("\n")[1]}`);
    })
}

const _global = (window /* browser */ || global /* node */);
_global.fluence = fluence;
_global.getCount = getCount;
_global.getMax = getMax;
_global.getData = getData;





