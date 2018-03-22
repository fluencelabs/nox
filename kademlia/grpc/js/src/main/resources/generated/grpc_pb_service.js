// package: fluence.kad
// file: grpc.proto

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

