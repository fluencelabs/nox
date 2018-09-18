#!/usr/bin/python
import sys, json, numpy, base64, binascii
if sys.version_info[0] == 2:
    from urllib import urlopen
else:
    from urllib.request import urlopen

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
	buf = b""
	while x >= 0x80:
		buf += ints_to_bytes([x & 0xFF | 0x80])
		x >>= 7
	buf += ints_to_bytes([x])
	return buf

def ints_to_bytes(ints):
	if sys.version_info[0] == 2:
		return "".join(map(chr, ints))
	else:
		return bytes(ints)

def l_endian_4b(num):
	return ints_to_bytes([num & 0xFF, (num & 0xFF00) >> 8, (num & 0xFF0000) >> 16, (num & 0xFF000000) >> 24])

def b64_decode(b64):
	return base64.b64decode(b64.encode()).decode()

def hex_encode(text):
	return binascii.b2a_hex(text.encode()).decode().upper()

def hex_encode_bytes(b):
	return binascii.b2a_hex(b).decode().upper()

def hex_decode(hex):
	return hex_decode_bytes(hex).decode()

def hex_decode_bytes(hex):
	return binascii.a2b_hex(hex.encode())

def parse_utc_unix_ns(timestamp_txt):
	dt64 = numpy.datetime64(timestamp_txt).astype("datetime64[ns]").astype("int")
	(t_unix, t_ns) = (dt64 // 1000000000, dt64 % 1000000000)
	return (t_unix, t_ns)

def read_json(url):
	response = urlopen(url)
	return json.loads(response.read())
