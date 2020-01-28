
var socket;

window.getNetwork = function() {
	let msg = {
		action: "GetNetworkState"
	}
	socket.send(JSON.stringify(msg))
}

window.sendMsg = function(dst, msg_str) {
	if (!msg_str) msg_str = "some data"
	let msg = {
		action: "Relay",
		peer_id: dst,
		data: msg_str
	}
	socket.send(JSON.stringify(msg));
}

/**
 *
 * @param peerId examples:
 *     			 	QmcYE4o3HCpotey8Xm87ArERDp9KMgagUnjtKBxuA5vcBY
 * 				 	QmUz5ziqFiwuPJnUZehrQ3EyzpHjp22FyQRNH9AxRxKPbp
 */
window.connectWs = function(peerId, port, host) {
	if (!port) port = 8888;
	if (!host) host = "localhost";
	socket = new WebSocket(`ws://${host}:${port}/ws?key=${peerId}`);

	socket.onopen = function() {
		console.log("Connection established.");
	};

	socket.onclose = function(event) {
		console.log("Connection closed");
	};

	socket.onmessage = function(event) {
		console.log("Received message " + event.data);
	};

	socket.onerror = function(error) {
		console.log("Error " + error.message);
	};

}
