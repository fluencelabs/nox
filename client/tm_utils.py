from misc_utils import read_json

def get_sync_info(tmaddress):
	status = read_json(tmaddress + "/status")["result"]
	if "sync_info" in status: # compatibility
		return status["sync_info"]
	else:
		return status

def get_max_height(tmaddress):
	return get_sync_info(tmaddress)["latest_block_height"]
