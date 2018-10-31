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

import sys
sys.path.append('..')

import ed25519, base64
from codec import b64_encode_bytes

def get_signing_key():
	return ed25519.SigningKey(b"TVAD4tNeMH2yJfkDZBSjrMJRbavmdc3/fGU2N2VAnxT3hAtSkX+Lrl4lN5OEsXjD7GGG7iEewSod472HudrkrA==", encoding="base64")

def get_seed():
	# TVAD4tNeMH2yJfkDZBSjrMJRbavmdc3/fGU2N2VAnxQ=
	return b64_encode_bytes(get_signing_key().to_seed())

def get_verifying_key():
	# 94QLUpF/i65eJTeThLF4w+xhhu4hHsEqHeO9h7na5Kw=
	return b64_encode_bytes(get_signing_key().get_verifying_key().to_bytes())

def get_client():
    return "client001"
