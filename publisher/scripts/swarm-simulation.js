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

http.createServer(function (request, response) {
    response.writeHead(200);
    response.end("d1f25a870a7bb7e5d526a7623338e4e9b8399e76df8b634020d11d969594f24a", 'utf-8');
}).listen(8500);
