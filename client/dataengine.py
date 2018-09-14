#!/usr/bin/python
import string, random, time
from verify import verify_commit, verify_app_hash, verify_merkle_proof

def id_generator(size = 6, chars = string.ascii_uppercase[:6] + string.digits):
	return ''.join(random.choice(chars) for _ in range(size))

def sign(message, sk):
	if sk is None:
		return ""
	else:
		return sk.sign(message, encoding="base64")

class DataEngineResultAwait:
	def __init__(self, session, target_key):
		self.session = session
		self.target_key = target_key

	def result(self, timeout = 5):
		tm = self.session.engine.tm
		path = self.target_key + "/result"
		print "querying " + path
		for _ in range(0, timeout):
			resp = tm.query(path)
			if "value" in resp:
				# to verify value we need to verify each transition here:
				# value => merkle_proof => app_hash => vote => commit => genesis

				check_height = int(resp["height"])
				app_hash = tm.get_block(check_height)["header"]["app_hash"]
				result = resp["value"].decode("base64")

				if not "proof" in resp:
					print "No proof for result"
					return None

				# value => merkle_proof => app_hash
				if not verify_merkle_proof(result, resp["proof"].decode("base64"), app_hash):
					print "Result proof failed"
					return None


				# app_hash => vote
				signed_header = tm.get_commit(check_height)["SignedHeader"]
				if not verify_app_hash(app_hash, signed_header):
					print "App hash verification failed"
					return None

				# vote => commit => genesis
				validators = tm.get_validators(check_height)
				commit_ver = verify_commit(signed_header, validators, check_height, self.session.engine.genesis)
				if commit_ver != "OK":
					print "Commit verification failed: " + commit_ver
					return None

				return result
			time.sleep(1)
		return None

class DataEngineSession:
	def __init__(self, engine, client = None, signing_key = None):
		if client == None:
			client = "anon"
		self.engine = engine
		self.client = client
		self.signing_key = signing_key
		self.session = id_generator()
		self.counter = 0

	def submit(self, command, *params):
		payload = "%s(%s)" % (command, ','.join(map(str, params)))
		tx_sign_bytes = "%s-%s-%d-%s" % (self.client, self.session, self.counter, payload)
		signature = sign(tx_sign_bytes, self.signing_key)
		tx_json = str({
			"tx": {
				"header": {
					"client": self.client,
					"session": self.session,
					"order": self.counter
				},
				"payload": payload
			},
			"signature": signature
		}).replace("'", '"')
		target_key = "@meta/%s/%s/%d" % (self.client, self.session, self.counter)
		print "submitting", tx_json
		self.engine.tm.broadcast_tx_sync(tx_json)
		self.counter += 1
		return DataEngineResultAwait(self, target_key)

class DataEngine:
	def __init__(self, tm, genesis):
		self.tm = tm
		self.genesis = genesis

	def new_session(self, client = None, signing_key = None):
		return DataEngineSession(self, client, signing_key)
