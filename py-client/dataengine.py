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
import string, random, time, hashlib
from verify import get_verified_result
from codec import hex_decode, hex_decode_bytes, hex_encode_bytes, le4b_decode

def id_generator(size = 6, chars = string.ascii_uppercase[:6] + string.digits):
	"""
	Generates a non-secure random `string` used as IDs for Data engine objects, e. g. sessions.

	Arguments:
		size
			The requested length of ID.
		chars
			Character set to use for generation.
	"""
	return ''.join(random.choice(chars) for _ in range(size))

def sign(message, private_key):
	"""
	Signs the given `message` with the given ed25519 private key.

	Arguments:
		message
			`string` to sign.
		private_key
			Private key (in base-64 `string`) used to sign the given data.
	"""
	return private_key.sign(message, encoding="base64").decode()

class DataEngineResultAwait:
	"""
	The awaitable Data engine submit result - used to asynchronously retrieve the results/errors of the invocation
	of a particular request.
	"""

	def __init__(self, session, target_key):
		"""
		Initializes the awaitable Data engine submit result.

		Arguments:
			session
				Session object containing the required data to retrieve results.
			target_key
				A particular key pointing to a location in the server state tree that is expected to store the results.
		"""
		self.session = session
		self.target_key = target_key

	def result(self, requests_per_sec = 2, response_timeout_sec = 10, wait_before_request_session_summary_sec = 2):
		"""
		Retrieves the invocation results of previously submitted request from the server.
		In case of success, interprets the result as string.

		Arguments:
			requests_per_sec
				Result retrieval rate.
			response_timeout_sec
				Number denoting how long (in seconds) the result retrieval attempts run.
			wait_before_request_session_summary_sec
				Number denoting when (in seconds since the retrieval start) the session summary is started to
				retrieve together with the result.				
		"""
		raw = self.raw_result(requests_per_sec, response_timeout_sec, wait_before_request_session_summary_sec)
		if (type(raw) is dict) and ("Computed" in raw):
			return hex_decode(raw["Computed"]["value"])
		else:
			return raw

	def result_num(self, requests_per_sec = 2, response_timeout_sec = 10, wait_before_request_session_summary_sec = 2):
		"""
		Retrieves the invocation results of previously submitted request from the server.
		In case of success, interprets the result as number.

		Arguments:
			requests_per_sec
				Result retrieval rate.
			response_timeout_sec
				Number denoting how long (in seconds) the result retrieval attempts run.
			wait_before_request_session_summary_sec
				Number denoting when (in seconds since the retrieval start) the session summary is started to
				retrieve together with the result.				
		"""
		raw = self.raw_result(requests_per_sec, response_timeout_sec, wait_before_request_session_summary_sec)
		if (type(raw) is dict) and ("Computed" in raw):
			return le4b_decode(hex_decode_bytes(raw["Computed"]["value"]))
		else:
			return raw

	def raw_result(self, requests_per_sec = 2, response_timeout_sec = 10, wait_before_request_session_summary_sec = 2):
		"""
		Retrieves the invocation results of previously submitted request from the server.

		Arguments:
			requests_per_sec
				Result retrieval rate.
			response_timeout_sec
				Number denoting how long (in seconds) the result retrieval attempts run.
			wait_before_request_session_summary_sec
				Number denoting when (in seconds since the retrieval start) the session summary is started to
				retrieve together with the result.				
		
		Returns:
			Either a validatated JSON representation of the result invocation call on the server's VM
			or an error message denoting some problem occurred during result retrieval or validation.

		Normally the result is ready in 1-2 seconds since the request submit. However, it may fail to be computed
		(for example, because the earlier request failed). Therefore, after the result is not ready for
		`wait_before_request_session_summary_sec` the session summary request is initiated to obtain the information
		about the whole session summary on the server side - it might contain more information.
		"""
		tm = self.session.engine.tm
		path = self.target_key + "/result"
		for i in range(0, response_timeout_sec * requests_per_sec):
			verified_session_summary = None
			if i >= wait_before_request_session_summary_sec * requests_per_sec:
				summary_response = tm.query(self.session.summary_key)
				if "value" in summary_response:
					verified_session_summary = get_verified_result(tm, self.session.engine.genesis, summary_response)
					
			query_response = tm.query(path)
			if "value" in query_response:
				return get_verified_result(tm, self.session.engine.genesis, query_response)

			if verified_session_summary is not None and "Active" not in verified_session_summary["status"]:
				return verified_session_summary
			time.sleep(1.0 / requests_per_sec)
		return "Result is not ready!"

class DataEngineSession:
	"""
	The Data engine session - class allowing a single client identity to submit authenticated ordered requests to
	the server and retrieve their results.
	"""

	def __init__(self, engine, client, signing_key, session):
		"""
		Initializes the Data engine Session object.

		Arguments:
			engine
				Data engine containing information how to connect the server and verify its responses.
			client
				Client ID.
			signing_key
				Client's private key to sign session's requests.
			session
				Session ID.
		"""
		self.engine = engine
		self.client = client
		self.signing_key = signing_key
		self.session = session
		self.summary_key = "@meta/%s/%s/@sessionSummary" % (self.client, self.session)
		self.counter = 0

	def submit(self, command, args):
		"""
		Submits a given `command` with given `params` to the server node.

		Arguments:
			command
				Function name to invoke on the server side.
			args
				The arguments of the called function as bytes.
		
		Returns:
			An awaitable `DataEngineResultAwait` that allows to retrieve the results of the function call.
		"""
		payload = "%s(%s)" % (command, hex_encode_bytes(args))
		tx_sign_text = "%s-%s-%d-%s" % (self.client, self.session, self.counter, payload)
		tx_unhashed_sign_bytes = tx_sign_text.encode()
		tx_sign_bytes = hashlib.sha256(tx_unhashed_sign_bytes).digest()
		signature = sign(tx_sign_bytes, self.signing_key)
		tx_json = str({
			"tx": {
				"header": {
					"client": self.client,
					"session": self.session,
					"order": self.counter
				},
				"payload": payload,
				"timestamp": int(round(time.time() * 1000))
			},
			"signature": signature
		}).replace("'", '"').replace('u"', '"')
		target_key = "@meta/%s/%s/%d" % (self.client, self.session, self.counter)
		
		tx_response = self.engine.tm.broadcast_tx_sync(tx_json)
		if "result" not in tx_response:
			print(tx_response["error"]["data"])
		elif tx_response["result"]["code"] != 0:
			print(hex_decode(tx_response["result"]["data"]))

		self.counter += 1
		return DataEngineResultAwait(self, target_key)

	def close(self):
		"""
		Closes the current session and makes it unable to submit new requests.

		Returns:
			An awaitable `DataEngineResultAwait`, similar to `submit` method.

		Implemented by `submit`ting a dedicated parameterless `@closeSession` function request.
		"""
		return self.submit("@closeSession", "".encode())

class DataEngine:
	"""
	The entry point to the Data engine API.
	"""

	def __init__(self, tm, genesis):
		"""
		Initializes the Data engine object.

		Arguments:
			tm
				`TendermintRPC` object served as transport layer proxy to access the server.
			genesis
				Tendermint genesis data acting as the trusted source of information during server response validation.
		"""
		self.tm = tm
		self.genesis = genesis

	def new_session(self, client, signing_key, session = None):
		"""
		Creates a new session on behalf of the given client.

		Arguments:
			client
				Client ID.
			signing_key
				Client's private key to sign session's requests.
			session
				Session ID. `None` would cause auto-generated ID.
		"""
		return DataEngineSession(self, client, signing_key, session if session is not None else id_generator())
