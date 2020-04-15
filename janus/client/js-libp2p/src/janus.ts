/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

import {connect, connectWithKey, JanusConnection} from "./janus_connection";
import {IpfsMultiaddrService} from "./ipfs_multiaddr_service";
import {IpfsShowService} from "./ipfs_show_service";
import {ipfsAdd, ipfsGet} from "./ipfs_service";
import {FunctionCall, genUUID} from "./function_call";

interface Server {
    peer: string,
    ip: string,
    port: number
}

function server(peer: string, ip: string, port: number): Server {
    return {
        peer: peer,
        ip: ip,
        port: port
    }
}

const servers = [

    server("QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz", "134.209.186.43", 9001),
    server("QmVzDnaPYN12QAYLDbGzvMgso7gbRD9FQqRvGZBfeKDSqW", "134.209.186.43", 9002),
    server("QmX6yYZd4iLW7YpmZz4waLrtb5Y9f5v3PPGEmNGh9k3iW2", "134.209.186.43", 9990)
    // server("QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz", "104.248.25.59", 9001),
    // server("QmVzDnaPYN12QAYLDbGzvMgso7gbRD9FQqRvGZBfeKDSqW", "104.248.25.59", 9002),
    // server("QmX6yYZd4iLW7YpmZz4waLrtb5Y9f5v3PPGEmNGh9k3iW2", "104.248.25.59", 9990)
];


const multiaddrService = "IPFS.multiaddr";

function changeConnectionStatus(status: string) {
    let statusEl = document.getElementById("connection-status") as HTMLSpanElement;
    statusEl.innerText = status;
}

function shortPeerId(peerId: string) {
    return peerId.substr(0, 4) + "..." + peerId.substr(peerId.length - 4, peerId.length)
}

function initDownloadButton() {
    let downloadButton = document.getElementById("download-submit") as HTMLInputElement;
    downloadButton.onclick = async function(e) {
        let hashInput = document.getElementById("download-hash") as HTMLInputElement;

        let hash = hashInput.value;
        let msgId = genUUID() + hash;
        if (hash && hash.length === 46) {
            connection.sendServiceCall("IPFS.get_" + hash, {msg_id: msgId}, "peer call IPFS.get_" + hash);
            connection.subscribe((call: FunctionCall) => {
                if (call.arguments.msg_id === msgId) {

                    console.log(call.arguments.multiaddr);

                    ipfsGet(call.arguments.multiaddr, hash).then(async (bytes) => {

                        const blob = new Blob([bytes], {type: "application/octet-stream"});
                        const link = document.createElement('a');
                        link.href = window.URL.createObjectURL(blob);
                        link.download = hash;
                        link.click();

                    }).catch((er) => {
                        console.log("ERROR");
                        console.log(er);
                    });

                    return true;
                }
            });
        }
    };
}

function initUploadButton() {
    let uploadButton = document.getElementById("upload-submit") as HTMLInputElement;
    uploadButton.onclick = async function(e) {
        let fileEl = document.getElementById("upload-file") as HTMLInputElement;
        let file = fileEl.files[0] as File;
        let reader = new FileReader();
        reader.readAsArrayBuffer(file);

        reader.onload = async function() {

            let arrayBuffer = this.result as ArrayBuffer;
            let array = new Uint8Array(arrayBuffer);

            let service = new IpfsShowService(connection, array, multiaddrService);

            console.log(new TextDecoder("utf-8").decode(array));

            let serviceName = await service.getName();
            let hash = await service.getHash();
            console.log("service name: " + serviceName);
            await connection.registerService(serviceName, service.service());

            let ul = document.getElementById("list-of-files");
            let li = document.createElement("li");
            li.appendChild(document.createTextNode(hash));
            ul.appendChild(li);

        };
    };
}

function initServersList(initNumber: number) {
    let serversListEl = document.getElementById("servers-select") as HTMLSelectElement;

    for (let i in servers) {
        let server = { ...servers[i] };
        let option = document.createElement("option") as HTMLOptionElement;
        server.peer = shortPeerId(server.peer);
        option.innerText = JSON.stringify(server);
        option.value = i;
        if (initNumber === parseInt(i)) {
            option.selected = true;
        }
        serversListEl.add(option);
    }

    serversListEl.onchange = function (e) {
        let optionNumber = (e.target as any).value;
        reconnect(servers[optionNumber]);
    };
}

async function createMultiaddrService() {
    let con = await connectWithKey(1, "QmVzDnaPYN12QAYLDbGzvMgso7gbRD9FQqRvGZBfeKDSqW", "104.248.25.59", 9002);
    let service = new IpfsMultiaddrService(con, "/dns4/ipfs1.fluence.one/tcp/5001");
    await con.registerService(multiaddrService, service.service());
}

async function createPeer(serviceName: string) {
    let con = await connectWithKey(2, "QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz", "104.248.25.59", 9001);

    await con.sendServiceCall(serviceName, {}, "peer call IPFS.get_...")
}

async function createShowService() {
    let con = await connectWithKey(3, "QmX6yYZd4iLW7YpmZz4waLrtb5Y9f5v3PPGEmNGh9k3iW2", "104.248.25.59", 9990);

    let service = new IpfsShowService(con, Buffer.from("12333333333333333333"), multiaddrService);

    let serviceName = await service.getName();
    console.log("service name: " + serviceName);
    await con.registerService(serviceName, service.service());
}

async function checkIPFS() {
    // let multiaddr = '/ip4/127.0.0.1/tcp/5001';
    let multiaddr = '/dns4/ipfs1.fluence.one/tcp/5001';

    let f: any = await ipfsAdd(multiaddr, new Uint8Array([1,2,3,4]));
    let res = await ipfsGet(multiaddr, f.path);

    console.log(res);
}

async function reconnect(server: Server) {

    if (connection) {
        await connection.disconnect();
        changeConnectionStatus("Connecting...");
    }

    connection = await connect(server.peer, server.ip, server.port);

    changeConnectionStatus("Connected");
}

window.onload = async function () {
    let serverNum = Math.floor(Math.random() * servers.length);

    initServersList(serverNum);

    let server = servers[serverNum];
    console.log("SERVER: " + JSON.stringify(server));
    connection = await connect(server.peer, server.ip, server.port);
    changeConnectionStatus("Connected");

    initDownloadButton();
    initUploadButton();

};

let connection: JanusConnection;


if (window && typeof window !== undefined) {
    (<any>window).connect = connect;
    (<any>window).connectWithKey = connectWithKey;
    (<any>window).createMultiaddrService = createMultiaddrService;
    (<any>window).createPeer = createPeer;
    (<any>window).createShowService = createShowService;
    (<any>window).checkIPFS = checkIPFS;
}

