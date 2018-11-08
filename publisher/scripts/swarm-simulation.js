var http = require('http');

process.title = process.title = "swarm-simulation";

http.createServer(function (request, response) {
    response.writeHead(200);
    response.end("d1f25a870a7bb7e5d526a7623338e4e9b8399e76df8b634020d11d969594f24a", 'utf-8');
}).listen(8500);
