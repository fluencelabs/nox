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
	dt64 = numpy.datetime64(timestamp_txt).astype("datetime64[ns]").astype("int")
	(t_unix, t_ns) = (dt64 // 1000000000, dt64 % 1000000000)
	return (t_unix, t_ns)

def read_json(url):
	response = urlopen(url)
	return json.loads(response.read())
