import StreamrClient from 'streamr-client'
import * as fluence from "js-fluence-client"

const streamrClient = new StreamrClient();
const fluenceSession = fluence.createDefaultSession("localhost", 29057);

const createTableQuery = "CREATE TABLE polution_uusimaa(id varchar(128), location varchar(128), parameter varchar(128), " +
    "value double, unit varchar(128), country varchar(128), city varchar(128), latitude double, " +
    "longitude double, local varchar(128), utc varchar(128))";

const deleteQuery = "DELETE from polution_uusimaa";

fluenceSession.invoke("do_query", deleteQuery).result().then((r) => console.log(r.asString()));
fluenceSession.invoke("do_query", createTableQuery).result().then((r) => console.log(r.asString()));

function insertQuery(data) {
    const query = `INSERT INTO polution_uusimaa VALUES ('${data.id}', '${data.location}', '${data.parameter}', ${data.value}, ` +
        `'${JSON.stringify(data.unit)}', '${data.country}', '${data.city}', ${data.latitude}, ${data.longitude}, '${data.local}', '${data.utc}')`;
    console.log("Query: " + query);
    return query;
}

function getCount() {
    const query = "select count(*) from polution_uusimaa";
    fluenceSession.invoke("do_query", query).result().then((r) => {
        console.log("Data count: " + r.asString().split("\n")[1])
    })
}

function getData(resendLast) {
    const sub = streamrClient.subscribe(
        {
            stream: 'dVoD8tzLR6KLJ-z_Pz8pMQ',
            resend_last: resendLast
        },
        (message, metadata) => {
            console.log(JSON.stringify(message));
            console.log(JSON.stringify(metadata));
            const query = insertQuery(message);
            fluenceSession.invoke("do_query", query);
        }
    )
}

/**
 * Get maximum value of parameter
 *
 * @param parameter [pm25, pm10, no2, o3]
 */
function getMax(parameter) {
    const query = `select max(value) from polution_uusimaa where parameter = '${parameter}'`;
    fluenceSession.invoke("do_query", query).result().then((r) => {
        console.log(`Maximum of ${parameter}: ${r.asString().split("\n")[1]}`);
    })
}

const _global = (window /* browser */ || global /* node */);
_global.fluence = fluence;
_global.getCount = getCount;
_global.getMax = getMax;
_global.getData = getData;





