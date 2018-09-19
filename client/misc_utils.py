#!/usr/bin/python
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
