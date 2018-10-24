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

import base64, time
from tmrpc import TendermintRPC
from dataengine import DataEngine, id_generator
from codec import le4b_encode
from client_identity import get_client, get_signing_key

def submit_query(session, query):
    return session.submit("do_query", query.encode())

def demo_llamadb(addr, genesis):
    eng = DataEngine(addr, genesis)
    s = eng.new_session(get_client(), get_signing_key())
    print(submit_query(s, "CREATE TABLE users(id int, name varchar(128), age int)").result())
    for i in range(0, 20):
        submit_query(s, "INSERT INTO users VALUES(%d, 'User%d', %d)" % (i, i, i + 20))
        print(submit_query(s, "SELECT AVG(age) FROM users").result())
    s.close()

tmport = sys.argv[1] if len(sys.argv) >= 2 else "25057"
tm = TendermintRPC("http://localhost:" + tmport)
genesis = tm.get_genesis()

demo_llamadb(tm, genesis)
