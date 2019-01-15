# Copyright 2018 Fluence Labs Limited
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# Import Fabric's API module
from fabric.api import *

# owners and private keys for specific ip addresses
# todo get this info from some sources
# TO USE: replace values inside <> with your actual values
info = {'<ip1>': {'owner': '<eth address1>', 'key': '<private key1>'},
        '<ip2>': {'owner': '<eth address2>', 'key': '<private key2>'}}

# Fluence will be deployed on all hosts from `info`
env.hosts = info.keys()

# Set the username
env.user = "root"
 
 
def copy_resources():

    # copy local directory `script` to remote machine
    put('scripts', '.')

# comment this annotation to deploy sequentially
@parallel
def deploy():
 
    copy_resources()

    with cd("scripts"):

        # change for another chain
        # todo changing this variable should recreate parity container
        # todo support contract deployment on 'dev' chain
        chain='kovan'

        # actual fluence contract address
        contract_address='0xd5D58ABc1839628b3EaC364B801Cf13D510Ad6a9'

        # getting owner and private key from `info` dictionary
        current_host = env.host_string
        current_owner = info[current_host]['owner']
        current_key = info[current_host]['key']

        with shell_env(CHAIN=chain,
                       REMOTE_DEPLOY="true",
                       CONTRACT_ADDRESS=contract_address,
                       BZZ_KEY=current_owner,
                       OWNER_ADDRESS=current_owner,
                       PORTS="25000:25003",
                       NAME="node1",
                       PRIVATE_KEY=current_key,
                       HOST_IP=current_host,
                       PARITY_ARGS='--light --chain kovan --jsonrpc-apis=all --jsonrpc-hosts=all --jsonrpc-cors="*" --unsafe-expose'):
            run('chmod +x compose.sh')
            run('chmod +x fluence')
            run('docker pull fluencelabs/node')
            run('docker pull fluencelabs/worker')
            run('./compose.sh')
