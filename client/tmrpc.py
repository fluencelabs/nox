from misc_utils import read_json

class TendermintRPC:
	def __init__(self, addr):
		self.addr = addr

	def result_from_json(self, template, *params):
		return read_json(self.addr + template % params)["result"]

	def broadcast_tx_sync(self, tx_json):
		return self.result_from_json('/broadcast_tx_sync?tx="%s"', tx_json.encode("hex").upper())

	def query(self, path):
		return self.result_from_json('/abci_query?path="%s"', path)["response"]

	def get_commit(self, height):
		return self.result_from_json("/commit?height=%d", height)

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
