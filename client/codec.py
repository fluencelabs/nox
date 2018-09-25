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
	"""
	Convert `int` to `bytes` object in unsigned varint (variable-length encoding) format.
	See: https://developers.google.com/protocol-buffers/docs/encoding

	Arguments:
		x
			`int` to convert to `uvarint`.
	"""
	buf = b""
	while x >= 0x80:
		buf += ints_to_bytes(x & 0xFF | 0x80)
		x >>= 7
	buf += ints_to_bytes(x)
	return buf

def ints_to_bytes(*ints):
	"""
	Helper method to convert `int`s to `bytes` object.

	Arguments:
		ints
			Vararg `int`s to be converted to `bytes`. Every item must be in [0..255] range.
	"""
	if sys.version_info[0] == 2:
		return "".join(map(chr, ints))
	else:
		return bytes(ints)

def l_endian_4b(num):
	"""
	Converts `int` to a little endian 4 byte `bytes` object.

	Arguments:
		num
			Integer to represent in the little endian format.
	"""
	return ints_to_bytes(num & 0xFF, (num & 0xFF00) >> 8, (num & 0xFF0000) >> 16, (num & 0xFF000000) >> 24)

def b64_decode(b64):
	"""
	Decodes `string` from base-64 `string` representation.

	Arguments:
		b64
			Base-64 text representation of encoded string.
	"""
	return base64.b64decode(b64.encode()).decode()

def hex_encode(text):
	"""
	Encodes `string` to hexademimal `string` representation.

	Arguments:
		text
			Source text to encode to hex.
	"""
	return hex_encode_bytes(text.encode())

def hex_encode_bytes(b):
	"""
	Encodes bynary data to hexademimal `string` representation.

	Arguments:
		text
			Source `bytes` to encode to hex.
	"""
	return binascii.b2a_hex(b).decode().upper()

def hex_decode(hex):
	"""
	Decodes `string` from hexademimal `string` representation.

	Arguments:
		hex
			Hexadecimal text representation of encoded string.
	"""
	return hex_decode_bytes(hex).decode()

def hex_decode_bytes(hex):
	"""
	Decodes `bytes` from hexademimal `string` representation.

	Arguments:
		hex
			Hexadecimal text representation of encoded data.
	"""
	return binascii.a2b_hex(hex.encode())

