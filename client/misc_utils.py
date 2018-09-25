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

import sys, json, numpy
if sys.version_info[0] == 2:
    from urllib import urlopen
else:
    from urllib.request import urlopen

def parse_utc_unix_ns(timestamp_txt):
	"""
	Parses a given `timestamp_txt` which is in ISO 8601 with nanoseconds, like
	.. 2018-09-25T04:34:45.123456789Z
	
	Arguments:
		timestamp_txt
			Time string in ISO 8601 format

	Returns an `int` tuple: `(UTC unix time, nanoseconds)`.
	"""
	dt64 = numpy.datetime64(timestamp_txt).astype("datetime64[ns]").astype("int")
	(t_unix, t_ns) = (dt64 // 1000000000, dt64 % 1000000000)
	return (t_unix, t_ns)

def read_json_from_url(url):
	"""
	Send a HTTP request to a given `url` and reads the JSON from the response.
	
	Arguments:
		url
			URL `string` denoting the target JSON location.
	"""
	response = urlopen(url)
	r = response.read()
	#trace = not any(["/" + x + "?" in url for x in ["genesis", "block", "commit", "validators"]])
	trace = False
	if trace:
		print()
		print("Request: " + url)
		print("Response: " + str(r))
	return json.loads(r)
