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

file = open("scripts/contract.txt", "r")
contract=file.read()
file.close()

# Fluence will be deployed on all hosts from `info`
env.hosts = info.keys()

# Set the username
env.user = "root"
 
def copy_resources():

    run('mkdir scripts -p')
    # copy local directory `script` to remote machine
    put('scripts/compose.sh', '.')
    put('scripts/node.yml', '.')
    put('scripts/parity.yml', '.')
    put('scripts/swarm.yml', '.')
    put('scripts/fluence', '.')

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
        contract_address=contract

        # getting owner and private key from `info` dictionary
        current_host = env.host_string
        current_owner = info[current_host]['owner']
        current_key = info[current_host]['key']

        with shell_env(CHAIN=chain,
                       PROD_DEPLOY="true",
                       CONTRACT_ADDRESS=contract_address,
                       OWNER_ADDRESS=current_owner,
                       PORTS="25000:25099",
                       NAME="node1",
                       PRIVATE_KEY=current_key,
                       HOST_IP=current_host):
            run('chmod +x compose.sh')
            run('chmod +x fluence')
            run('docker pull fluencelabs/node')
            run('docker pull fluencelabs/worker')
            # delete all workers
            # TODO reuse old volumes if possible
            run('docker ps -a | grep _node | awk \'{print $1}\' | xargs docker rm -f ; docker volume prune -f')
            run('./compose.sh')
