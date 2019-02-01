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
info = {'159.89.17.35': {'owner': '0x5903730e872fb2b0cd4402c69d6d43c86e973db7', 'key': '0x52c685b72c548da46ee7e595b4003635a1dab3f281dc26b9a13a5b0ea736d3b1', "ports": "25000:25099"}}

RELEASE="http://dump.bitcheese.net/files/refamix/fluence" #"https://github.com/fluencelabs/fluence/releases/download/untagged-3f7e10bd802b3149036d/fluence-linux-x64"

file = open("scripts/contract.txt", "r")

# gets deployed contract address from a file
contract=file.read().rstrip()
file.close()

# Fluence will be deployed on all hosts from `info`
env.hosts = info.keys()

# Set the username
env.user = "root"

# copies all necessary files for deploying
def copy_resources():
    # cleans up old scripts
    run('rm -rf scripts')
    run('mkdir scripts -p')
    # copy local directory `script` to remote machine
    put('scripts/compose.sh', 'scripts/')
    put('scripts/node.yml', 'scripts/')
    put('scripts/parity.yml', 'scripts/')
    put('scripts/swarm.yml', 'scripts/')

# creates register command to register deployed node
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

    # check if `fluence` file is exists
    result = local("[ -s fluence ] && echo 1 || echo 0", capture=True)
    if (result == '0'):
        print
        # todo: add correct link to CLI
        print '`fluence` CLI file does not exist. Downloading it from ' + RELEASE
        local("wget " + RELEASE)

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
                       # flag that show to script, that it will deploy all with non-default arguments
                       PROD_DEPLOY="true",
                       CONTRACT_ADDRESS=contract_address,
                       OWNER_ADDRESS=current_owner,
                       PORTS=current_ports,
                       # container name
                       NAME="fluence-node-1",
                       HOST_IP=current_host):
            run('chmod +x compose.sh')
            # the script will return command with arguments that will register node in Fluence contract
            output = run('./compose.sh deploy')
            meta_data = output.stdout.splitlines()[-1]
            # parses output as arguments in JSON
            json_data = json.loads(meta_data)
            # creates command for registering node
            command = register_command(json_data, current_key)
            # run `fluence` command
            local(command)

