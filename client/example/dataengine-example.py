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
from tmrpc import TendermintRPC
from dataengine import DataEngine, id_generator
from codec import le4b_encode

def get_signing_key():
	return ed25519.SigningKey(b"TVAD4tNeMH2yJfkDZBSjrMJRbavmdc3/fGU2N2VAnxT3hAtSkX+Lrl4lN5OEsXjD7GGG7iEewSod472HudrkrA==", encoding="base64")

def get_verifying_key():
	# 94QLUpF/i65eJTeThLF4w+xhhu4hHsEqHeO9h7na5Kw=
	get_signing_key.get_verifying_key()

def get_client():
    return "client001"

def submit_inc(session):
    return session.submit("inc", bytes(""))

def submit_get(session):
    return session.submit("get", bytes(""))

def submit_mul(session, f1, f2):
    return session.submit("MulModule.mul", le4b_encode(f1) + le4b_encode(f2))

def submit_wrong_command(session):
    return session.submit("wrong", bytes(""))

def demo_queries(addr, genesis, send_wrong=False, send_closed=True, session=None):
    eng = DataEngine(addr, genesis)
    s = eng.new_session(get_client(), get_signing_key(), session)
    q0 = submit_inc(s)
    q1 = submit_mul(s, 10, 14)
    if send_wrong:
        qw = submit_wrong_command(s)
    q2 = submit_inc(s)
    q3 = submit_get(s)
    if send_closed:
        closed = s.close()
    print(q1.result_num())
    print(q2.result())
    print(q3.result_num())
    if send_closed:
        print(closed.result())

def demo_many_queries(addr, genesis):
    eng = DataEngine(addr, genesis)
    s = eng.new_session(get_client(), get_signing_key())
    for _ in range(0, 500):
        submit_inc(s)
    print(submit_get(s).result_num())

tmport = sys.argv[1] if len(sys.argv) >= 2 else "25057"
tm = TendermintRPC("http://localhost:" + tmport)
genesis = tm.get_genesis()
height = tm.get_max_height()

# 1st session: correct, but not explicitly closed
session1_id = id_generator()
demo_queries(tm, genesis, False, False, session1_id)

# 2nd session: failed during processing
#demo_queries(tm, genesis, True, False)

# 3rd session: correct and explicitly closed
# 1st session expires during 3rd session processing
#demo_queries(tm, genesis, False, True)

# 4th session: same as 1st - transactions declined as duplicated
#demo_queries(tm, genesis, False, False, session1_id)

#demo_many_queries(tm, genesis)
