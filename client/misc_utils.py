#!/usr/bin/python
import sys, urllib, json, datetime, time

def from_uvarint(buf):
	x = long(0)
	s = 0
	for b in buf:
		if b < 0x80:
			return x | long(b) << s
		x |= long(b & 0x7f) << s
		s += 7
	return 0

def to_uvarint(x):
	buf = ""
	while x >= 0x80:
		buf += chr(x & 0xFF | 0x80)
		x >>= 7
	buf += chr(x)
	return buf

def parse_utc_unix_ns(utctxt):
	#tz conversion may be wrong
	now_timestamp = time.time()
	offset = datetime.datetime.fromtimestamp(now_timestamp) - datetime.datetime.utcfromtimestamp(now_timestamp)

	dt, _, tail = utctxt.partition(".")
	if tail == "":
		dt, _, _ = utctxt.partition("Z")
		tail = "0Z"
	t_unix = long((datetime.datetime.strptime(dt, '%Y-%m-%dT%H:%M:%S') + offset).strftime("%s"))
	t_ns = int(tail.rstrip("Z").ljust(9, "0"))
	return (t_unix, t_ns)

def read_json(url):
	response = urllib.urlopen("http://" + url)
	return json.loads(response.read())
