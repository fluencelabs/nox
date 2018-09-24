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

from __future__ import print_function
import string, random, time
from verify import get_verified_result
from codec import hex_decode

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

	def result(self, requests_per_sec = 2, response_timeout_sec = 10, wait_before_request_session_summary_sec = 2):
		tm = self.session.engine.tm
		path = self.target_key + "/result"
		print("querying " + path)
		for i in range(0, response_timeout_sec * requests_per_sec):
			verified_session_summary = None
			if i >= wait_before_request_session_summary_sec * requests_per_sec:
				summary_response = tm.query(self.session.summary_key)
				if "value" in summary_response:
					verified_session_summary = get_verified_result(tm, self.session.engine.genesis, summary_response)
					
			query_response = tm.query(path)
			if "value" in query_response:
				return get_verified_result(tm, self.session.engine.genesis, query_response)

			if verified_session_summary != None and not "Active" in verified_session_summary["status"]:
				return verified_session_summary
			time.sleep(1.0 / requests_per_sec)
		return None

class DataEngineSession:
	def __init__(self, engine, client, signing_key, session):
		self.engine = engine
		self.client = client
		self.signing_key = signing_key
		self.session = session
		self.summary_key = "@meta/%s/%s/@sessionSummary" % (self.client, self.session)
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
		tx_response = self.engine.tm.broadcast_tx_sync(tx_json)
		if not "result" in tx_response:
			print(tx_response["error"]["data"])
		elif tx_response["result"]["code"] != 0:
			print(hex_decode(tx_response["data"]))
		else:
			print("OK")

		self.counter += 1
		return DataEngineResultAwait(self, target_key)

	def close(self):
		return self.submit("@closeSession")

class DataEngine:
	def __init__(self, tm, genesis):
		self.tm = tm
		self.genesis = genesis

	def new_session(self, client, signing_key, session = None):
		return DataEngineSession(self, client, signing_key, session if session is not None else id_generator())
