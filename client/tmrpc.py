from misc_utils import read_json, hex_encode

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
