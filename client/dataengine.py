#!/usr/bin/python
from __future__ import print_function
import string, random, time
from verify import get_verified_result

def id_generator(size = 6, chars = string.ascii_uppercase[:6] + string.digits):
	return ''.join(random.choice(chars) for _ in range(size))

def sign(message, sk):
	if sk is None:
		return ""
	else:
		return sk.sign(message, encoding="base64").decode()

class DataEngineResultAwait:
	def __init__(self, session, target_key):
		self.session = session
		self.target_key = target_key

	def result(self, requests_per_sec = 2, response_timeout_sec = 10, wait_before_request_session_status_sec = 2):
		tm = self.session.engine.tm
		path = self.target_key + "/result"
		print("querying " + path)
		for i in range(0, response_timeout_sec * requests_per_sec):
			verified_session_status = None
			if i >= wait_before_request_session_status_sec * requests_per_sec:
				status_response = tm.query(self.session.status_key)
				if "value" in status_response:
					verified_session_status = get_verified_result(tm, self.session.engine.genesis, status_response)
					
			query_response = tm.query(path)
			if "value" in query_response:
				return get_verified_result(tm, self.session.engine.genesis, query_response)

			if verified_session_status != None and not "Active" in verified_session_status["status"]:
				return verified_session_status
			time.sleep(1.0 / requests_per_sec)
		return None

class DataEngineSession:
	def __init__(self, engine, client = None, signing_key = None):
		if client == None:
			client = "anon"
		self.engine = engine
		self.client = client
		self.signing_key = signing_key
		self.session = id_generator()
		self.status_key = "@meta/%s/%s/@session_status" % (self.client, self.session)
		self.counter = 0

	def submit(self, command, *params):
		payload = "%s(%s)" % (command, ",".join(map(str, params)))
		tx_sign_bytes = ("%s-%s-%d-%s" % (self.client, self.session, self.counter, payload)).encode()
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
		}).replace("'", '"').replace('u"', '"')
		target_key = "@meta/%s/%s/%d" % (self.client, self.session, self.counter)
		print("submitting", tx_json)
		self.engine.tm.broadcast_tx_sync(tx_json)
		self.counter += 1
		return DataEngineResultAwait(self, target_key)

	def close(self):
		return self.submit("@close_session")

class DataEngine:
	def __init__(self, tm, genesis):
		self.tm = tm
		self.genesis = genesis

	def new_session(self, client = None, signing_key = None):
		return DataEngineSession(self, client, signing_key)
