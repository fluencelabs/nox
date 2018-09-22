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

from misc_utils import read_json
from codec import hex_encode

class TendermintRPC:
	def __init__(self, addr):
		self.addr = addr

	def result_from_json(self, template, *params):
		return read_json(self.addr + template % params)["result"]

	def broadcast_tx_sync(self, tx_json):
		return self.result_from_json('/broadcast_tx_sync?tx="%s"', hex_encode(tx_json))

	def query(self, path):
		return self.result_from_json('/abci_query?path="%s"', path)["response"]

	def get_commit(self, height):
		result = self.result_from_json("/commit?height=%d", height)
		if "SignedHeader" in result: # TODO: undo after migration to 0.24.0
			return result["SignedHeader"]
		else:
			return result["signed_header"]

	def get_block(self, height):
		return self.result_from_json("/block?height=%d", height)["block"]

	def get_validators(self, height):
		return self.result_from_json("/validators?height=%d", height)["validators"]

	def get_genesis(self):
		return self.result_from_json("/genesis")["genesis"]

	def get_status(self):
		return self.result_from_json("/status")

	def get_max_height(self):
		return self.get_status()["sync_info"]["latest_block_height"]
