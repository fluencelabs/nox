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

import sys, base64, binascii

def to_uvarint(x):
	buf = b""
	while x >= 0x80:
		buf += ints_to_bytes(x & 0xFF | 0x80)
		x >>= 7
	buf += ints_to_bytes(x)
	return buf

def ints_to_bytes(*ints):
	if sys.version_info[0] == 2:
		return "".join(map(chr, ints))
	else:
		return bytes(ints)

def l_endian_4b(num):
	return ints_to_bytes(num & 0xFF, (num & 0xFF00) >> 8, (num & 0xFF0000) >> 16, (num & 0xFF000000) >> 24)

def b64_decode(b64):
	return base64.b64decode(b64.encode()).decode()

def hex_encode(text):
	return hex_encode_bytes(text.encode())

def hex_encode_bytes(b):
	return binascii.b2a_hex(b).decode().upper()

def hex_decode(hex):
	return hex_decode_bytes(hex).decode()

def hex_decode_bytes(hex):
	return binascii.a2b_hex(hex.encode())

