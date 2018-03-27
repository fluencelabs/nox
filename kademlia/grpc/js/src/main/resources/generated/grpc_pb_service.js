// package: fluence.kad
// file: grpc.proto

/*
    generated with protoc
    protoc --plugin=protoc-gen-js_service=./node_modules/.bin/protoc-gen-js_service \
    --js_out=import_style=commonjs,binary:generated \
    --js_service_out=generated -I protobuf protobuf/\*
    do not forget to regenerate the code if grpc.proto is changes
    TODO do it automatically
*/

var jspb = require("google-protobuf");
var grpc_pb = require("./grpc_pb");
var Kademlia = {
  serviceName: "fluence.kad.Kademlia"
};
Kademlia.ping = {
  methodName: "ping",
  service: Kademlia,
  requestStream: false,
  responseStream: false,
  requestType: grpc_pb.PingRequest,
  responseType: grpc_pb.Node
};
Kademlia.lookup = {
  methodName: "lookup",
  service: Kademlia,
  requestStream: false,
  responseStream: false,
  requestType: grpc_pb.LookupRequest,
  responseType: grpc_pb.NodesResponse
};
Kademlia.lookupAway = {
  methodName: "lookupAway",
  service: Kademlia,
  requestStream: false,
  responseStream: false,
  requestType: grpc_pb.LookupAwayRequest,
  responseType: grpc_pb.NodesResponse
};
module.exports = {
  Kademlia: Kademlia,
};

