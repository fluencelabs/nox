#!/usr/bin/python
import sys, urllib, json, numpy

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

def l_endian_4b(num):
	return chr(num & 0xFF) + chr((num & 0xFF00) >> 8) + chr((num & 0xFF0000) >> 16) + chr((num & 0xFF000000) >> 24)

def parse_utc_unix_ns(timestamp_txt):
	dt64 = numpy.datetime64(timestamp_txt).astype('datetime64[ns]').astype('int')
	(t_unix, t_ns) = (dt64 / 1000000000, dt64 % 1000000000)
	return (t_unix, t_ns)

def read_json(url):
	response = urllib.urlopen("http://" + url)
	return json.loads(response.read())
