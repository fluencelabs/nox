#!/usr/bin/python
import sys, json
import ed25519
import hashlib
if sys.version_info[0] == 2:
    import sha3
from misc_utils import to_uvarint, l_endian_4b, parse_utc_unix_ns, b64_decode, hex_decode, hex_decode_bytes, hex_encode, hex_encode_bytes, ints_to_bytes

# https://github.com/tendermint/tendermint/blob/master/types/vote.go
# https://github.com/tendermint/tendermint/blob/master/types/vote_test.go
def sign_bytes(vote, chain_id):
	tmpl = '{"@chain_id":"%s","@type":"vote","block_id":{"hash":"%s","parts":{"hash":"%s","total":"%s"}},"height":"%s","round":"%s","timestamp":"%s","type":2}'
	return (tmpl \
		% (chain_id, vote["block_id"]["hash"], vote["block_id"]["parts"]["hash"], vote["block_id"]["parts"]["total"], \
		vote["height"], vote["round"], vote["timestamp"])).encode()

# https://github.com/tendermint/tendermint/blob/master/types/validator_set.go
def verify_commit(signed_header, validators, height, genesis):
	header = signed_header["header"]
	commit = signed_header["commit"]
	precommit1 = [x for x in commit["precommits"] if x is not None][0]
	if len(genesis["validators"]) != len(commit["precommits"]):
		return "Invalid commit -- wrong set size: %d vs %d" % (len(genesis["validators"]), len(commit["precommits"]))
	if int(header["height"]) != height:
		return "Invalid commit -- wrong height: %s vs %d" % (header["height"], height)
	tallied_power = 0
	for precommit in commit["precommits"]:
		if precommit is None:
			continue
		idx = int(precommit["validator_index"])
		if int(precommit["height"]) != height:
			return "Invalid commit -- wrong height: %s vs %d" % (precommit["height"], height)
		if precommit["round"] != precommit1["round"]:
			return "Invalid commit -- wrong round: %s vs %s" % (precommit["round"], precommit1["round"])
		if precommit["type"] != 2:
			return "Invalid commit -- not precommit @ index %d" % (idx)

		validator = validators[idx]
		if len([x for x in genesis["validators"] if x["pub_key"] == validator["pub_key"] and x["power"] == validator["voting_power"]]) != 1:
			return "Invalid validator %d: not present in genesis or power changed" % (idx)

		vk = ed25519.VerifyingKey(validator["pub_key"]["value"], encoding="base64")
		signature = precommit["signature"]
		signbytes = sign_bytes(precommit, genesis["chain_id"])
		try:
			vk.verify(signature, signbytes, encoding="base64")
		except ed25519.BadSignatureError as e:
			return "Invalid commit -- invalid signature @ index %d: %s" % (idx, signature)

		if precommit["block_id"] != commit["block_id"]:
			continue # not error
		tallied_power += int(validator["voting_power"])
	total_power = sum(int(val["power"]) for val in genesis["validators"])
	if 3 * tallied_power <= 2 * total_power:
		return "Invalid commit -- insufficient voting power: got %d, needed >%0.1f" % (tallied_power, total_power * 2 / 3)
	return "OK"

def encode_slice(data):
	return to_uvarint(len(data)) + data

def digest(b):
	return hashlib.sha256(b).digest()[0:20]

# https://github.com/mappum/js-tendermint/blob/master/src/types.js
def hash_binary(data, format):
	if data == "":
		b = data.encode()
	elif format == "str":
		b = encode_slice(data.encode())
	elif format == "hex":
		b = encode_slice(hex_decode_bytes(data))
	elif format == "long":
		b = to_uvarint(int(data) * 2)
	elif format == "time":
		t_unix, t_ns = parse_utc_unix_ns(data)
		b = ints_to_bytes((1 << 3) | 1) + l_endian_4b(t_unix) + ints_to_bytes(0, 0, 0, 0, (2 << 3) | 5) + l_endian_4b(t_ns)
	elif format == "block_id":
		b = hex_decode_bytes("0A14" + data[0] + "121808021214" + data[1])
	else:
		raise Exception("Unknown format string: " + format)
	return digest(b)

def hash2(data1, data2):
	return digest(encode_slice(data1) + encode_slice(data2))

def simple_tree_hash(kvs):
	size = len(kvs)
	if size == 0:
		return None
	elif size == 1:
		(key, value) = kvs[0]
		return hash2(key.encode(), value)
	else:
		mid = (size + 1) // 2
		return hash2(simple_tree_hash(kvs[:mid]), simple_tree_hash(kvs[mid:]))

def verify_app_hash(app_hash, signed_header):
	header = signed_header["header"]
	if app_hash != header["app_hash"]:
		return False

	block_hash = signed_header["commit"]["block_id"]["hash"]
	# https://github.com/tendermint/tendermint/blob/master/types/block.go
	# https://github.com/tendermint/tendermint/blob/master/types/part_set.go
	# https://github.com/tendermint/go-amino/blob/master/encoder.go
	d = [
		("App",         hash_binary(header["app_hash"], "hex")),
		("ChainID",     hash_binary(header["chain_id"], "str")),
		("Consensus",   hash_binary(header["consensus_hash"], "hex")),
		("Data",        hash_binary(header["data_hash"], "hex")),
		("Evidence",    hash_binary(header["evidence_hash"], "hex")),
		("Height",      hash_binary(header["height"], "long")),
		("LastBlockID", hash_binary((header["last_block_id"]["hash"], header["last_block_id"]["parts"]["hash"]), "block_id")),
		("LastCommit",  hash_binary(header["last_commit_hash"], "hex")),
		("NumTxs",      hash_binary(header["num_txs"], "long")),
		("Results",     hash_binary(header["last_results_hash"], "hex")),
		("Time",        hash_binary(header["time"], "time")),
		("TotalTxs",    hash_binary(header["total_txs"], "long")),
		("Validators",  hash_binary(header["validators_hash"], "hex"))
	]
	tree_hash = hex_encode_bytes(simple_tree_hash(d))
	return tree_hash == block_hash

def verify_merkle_proof(result, proof, app_hash):
	parts = proof.split(", ")
	parts_len = len(parts)
	for index in range(parts_len, -1, -1):
		low_string = parts[index] if index < parts_len else result
		low_hash = hashlib.sha3_256(low_string.encode()).hexdigest().upper()
		high_hashes = parts[index - 1].split(" ") if index > 0 else [app_hash]
		if not any(low_hash in s for s in high_hashes):
			return False
	return True

def get_verified_result(tm, genesis, response):
	# to verify value we need to verify each transition here:
	# value => merkle_proof => app_hash => vote => commit => genesis

	check_height = int(response["height"])
	app_hash = tm.get_block(check_height)["header"]["app_hash"]
	result = b64_decode(response["value"])

	if not "proof" in response:
		print("No proof for result")
		return None

	# value => merkle_proof => app_hash
	if not verify_merkle_proof(result, b64_decode(response["proof"]), app_hash):
		print("Result proof failed")
		return None

	# app_hash => vote
	signed_header = tm.get_commit(check_height)
	if not verify_app_hash(app_hash, signed_header):
		print("App hash verification failed")
		return None

	# vote => commit => genesis
	validators = tm.get_validators(check_height)
	commit_ver = verify_commit(signed_header, validators, check_height, genesis)
	if commit_ver != "OK":
		print("Commit verification failed: " + commit_ver)
		return None

	return json.loads(result)
