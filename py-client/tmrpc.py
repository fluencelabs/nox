#!/usr/bin/python
"""
Copyright 2018 Fluence Labs Limited

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
"""

from misc_utils import read_json_from_url
from codec import hex_encode

class TendermintRPC:
	"""
	A transport layer proxy to access the server via Tendermint RPC calls.
	See: https://tendermint.com/rpc/#get-the-list
	"""

	def __init__(self, addr):
		"""
		Initializes the Tendermint RPC proxy.

		Arguments:
			addr
				The address of the root Tendermint RPC endpoint.
		"""
		self.addr = addr

	def json_from_request(self, template, *params):
		"""
		Base method to make Tendermint RPC calls.
		Return the JSON object from the location described by the arguments.

		Arguments:
			template
				The template `string` of a particular RPC method.
			params
				Vararg params to fill in `template`.
		"""
		return read_json_from_url(self.addr + template % params)

	def json_result_from_request(self, template, *params):
		"""
		Base method to make Tendermint RPC calls. Return a `result` field value
		from the JSON object from the location described by the arguments.

		Arguments:
			template
				The template `string` of a particular RPC method.
			params
				Vararg params to fill in `template`.
		"""
		return read_json_from_url(self.addr + template % params)["result"]

	def broadcast_tx_sync(self, tx_json):
		"""
		Submits the given `tx_json` data using Tendermint `broadcast_tx_sync`.
		This method waits for Tendermint mempool checking
		(including the call of `CheckTx` on the server State machine application)
		and returns a response, possible containing some error messages.

		Arguments:
			tx_json
				Tendermint transaction in JSON format.
		"""
		return self.json_from_request('/broadcast_tx_sync?tx="%s"', hex_encode(tx_json))

	def broadcast_tx_async(self, tx_json):
		"""
		Submits the given `tx_json` data using Tendermint `broadcast_tx_async`.
		This method doesn't wait for any Tendermint checkings.

		Arguments:
			tx_json
				Tendermint transaction in JSON format.
		"""
		return self.json_from_request('/broadcast_tx_async?tx="%s"', hex_encode(tx_json))

	def query(self, path):
		"""
		Makes the `abci_query` RPC call.
		The requested path is passed by Tendermint to the State machine which is in charge of
		providing the value on the requested `path` with a Merkle proof.

		Arguments:
			path
				Path denoting query's target location in the State tree.
		"""
		return self.json_result_from_request('/abci_query?path="%s"', path)["response"]

	def get_commit(self, height):
		"""
		Retrieves `signed_header` structure prepared by `commit` RPC.
		This structure is essentially `height`-th Tendermint block header
		with signatures (either contained in `height+1`-th block
		or to be contained in the next block produced by Tendermint).

		Arguments:
			height
				Height of the requested block.
		"""
		result = self.json_result_from_request("/commit?height=%d", height)
		if "SignedHeader" in result: # TODO: undo after migration to 0.24.0
			return result["SignedHeader"]
		else:
			return result["signed_header"]

	def get_block(self, height):
		"""
		Retrieves the `height`-th block from Tendermint.

		Arguments:
			height
				Height of the requested block.
		"""
		return self.json_result_from_request("/block?height=%d", height)["block"]

	def get_validators(self, height):
		"""
		Retrieves the `height`-th block's validators from Tendermint.

		Arguments:
			height
				Height of the requested block.
		"""
		return self.json_result_from_request("/validators?height=%d", height)["validators"]

	def get_genesis(self):
		"""
		Retrieves the Tendermint genesis data. This data includes:
		* Tendermint cluster `chain_id`
		* initial state
		* initial validator set with public keys and voting powers

		Note that this method should only be called on the trusted Tendermint node.
		It's purposelessly to retrieve this information from the node whose responses are verified.
		"""
		return self.json_result_from_request("/genesis")["genesis"]

	def get_status(self):
		"""
		Retrieves the Tendermint status. In particular, it contains the latest height.
		"""
		return self.json_result_from_request("/status")

	def get_max_height(self):
		"""
		Retrieves the latest committed height from Tendermint.
		"""
		return self.get_status()["sync_info"]["latest_block_height"]
