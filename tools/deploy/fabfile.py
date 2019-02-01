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
import json

# owners and private keys for specific ip addresses
# todo get this info from some sources
# TO USE: replace values inside <> with your actual values
info = {'<ip1>': {'owner': '<eth address1>', 'key': '<private key1>', 'ports': '<from>:<to>'},
        '<ip2>': {'owner': '<eth address2>', 'key': '<private key2>', 'ports': '<from>:<to>'}}

RELEASE="http://dump.bitcheese.net/files/refamix/fluence" #"https://github.com/fluencelabs/fluence/releases/download/untagged-3f7e10bd802b3149036d/fluence-linux-x64"

file = open("scripts/contract.txt", "r")
contract=file.read().rstrip()
file.close()

# Fluence will be deployed on all hosts from `info`
env.hosts = info.keys()

# Set the username
env.user = "root"

def copy_resources():

    # cleans up old scripts
    run('rm -rf scripts')
    run('mkdir scripts -p')
    # copy local directory `script` to remote machine
    put('scripts/compose.sh', 'scripts/')
    put('scripts/node.yml', 'scripts/')
    put('scripts/parity.yml', 'scripts/')
    put('scripts/swarm.yml', 'scripts/')

def register_command(data, secret_key):
    eth_url = "http://" + data['node_ip'] + ":8545"
    command = "./fluence register \
        --node_ip            %s \
        --tendermint_key     %s \
        --tendermint_node_id %s \
        --contract_address   %s \
        --account            %s \
        --secret_key         %s \
        --start_port         %s \
        --last_port          %s \
        --eth_url            %s \
        --wait_syncing \
        --base64_tendermint_key" % (data['node_ip'], data['tendermint_key'], data['tendermint_node_id'], data['contract_address'],
                                    data['account'], secret_key, data['start_port'], data['last_port'], eth_url)

    return command


# comment this annotation to deploy sequentially
@parallel
def deploy():

    result = local("[ -s fluence ] && echo 1 || echo 0", capture=True)
    if (result == '0'):
        print
        # todo: add correct link to CLI
        print '`fluence` CLI file does not exist. Download it from here https://github.com/fluencelabs/fluence/tree/master/cli'
    else:
        print result.stdout
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
            current_ports = info[current_host]['ports']

            with shell_env(CHAIN=chain,
                           PROD_DEPLOY="true",
                           CONTRACT_ADDRESS=contract_address,
                           OWNER_ADDRESS=current_owner,
                           PORTS=current_ports,
                           NAME="fluence-node-1",
                           HOST_IP=current_host):
                run('chmod +x compose.sh')
                # download fluence CLI
                output = run('./compose.sh')
                # the script will return command with arguments that will register node in Fluence contract
                # TODO return all arguments instead of the command itself or make a file or an output with all common commands
                meta_data = output.stdout.splitlines()[-1]
                json_data = json.loads(meta_data)
                command = register_command(json_data, current_key)
                local(command)

