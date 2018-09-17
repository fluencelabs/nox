#!/usr/bin/python
import sys
sys.path.append('..')

import ed25519, base64
from tmrpc import TendermintRPC
from dataengine import DataEngine

def get_signing_key():
	return ed25519.SigningKey(b"TVAD4tNeMH2yJfkDZBSjrMJRbavmdc3/fGU2N2VAnxT3hAtSkX+Lrl4lN5OEsXjD7GGG7iEewSod472HudrkrA==", encoding="base64")

def get_verifying_key():
	# 94QLUpF/i65eJTeThLF4w+xhhu4hHsEqHeO9h7na5Kw=
	get_signing_key.get_verifying_key()

def get_client():
    return "client001"

def demo_queries(addr, genesis):
    eng = DataEngine(addr, genesis)
    s = eng.new_session(get_client(), get_signing_key())
    q0 = s.submit("inc")
    q1 = s.submit("multiplier.mul", 10, 14)
    #qw = s.submit("wrong")
    q2 = s.submit("inc")
    q3 = s.submit("get")
    #closed = s.close()
    print q1.result()
    print q2.result()
    print q3.result()
    #print closed.result()

tm = TendermintRPC("localhost:46157")
genesis = tm.get_genesis()
height = tm.get_max_height()
demo_queries(tm, genesis)
