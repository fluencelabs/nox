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

var http = require('http');

process.title = process.title = "swarm-simulation";

var crypto = require('crypto')

let codes = new Map();

function randomValueHex(len) {
  return crypto
    .randomBytes(Math.ceil(len / 2))
    .toString('hex') // convert to hexadecimal format
    .slice(0, len) // return required number of characters
}

http.createServer(function (request, response) {
    let body;
    if(request.method === "POST") {
        request.on('data', chunk => {
                body += chunk;
            });
        request.on('end', function() {
            let hex = randomValueHex(64);
            codes.set(hex, body);
            response.writeHead(200);
            response.end(hex, 'utf-8');
        });
    } else if (request.method === "GET") {
        let pieces = request.url.split("/");
        response.writeHead(200);
        response.setHeader('Content-Type', 'application/octet-stream');
        response.setHeader('Content-Length', Buffer.byteLength(body));
        response.end(codes.get(pieces[pieces.length-1]))
    };

}).listen(8500);
