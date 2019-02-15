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

from fabric.api import *
import json
import utils

if hasattr(env, 'environment'):
    environment = env.environment

    # gets deployed contract address from a file
    file = open("deploy_config.json", "r")
    info_json = file.read().rstrip()
    file.close()

    info = json.loads(info_json)[environment]

    contract = info['contract']

    # Fluence will be deployed on all hosts in an environment from `info.json`
    nodes = info['nodes']
else:
    # gets deployed contract address from a file
    file = open("instances.json", "r")
    nodes_json = file.read().rstrip()
    file.close()

    file = open("scripts/contract.txt", "r")
    contract=file.read().rstrip()
    file.close()

    nodes = json.loads(nodes_json)

env.hosts = nodes.keys()

# Set the username
env.user = "root"

RELEASE="https://github.com/fluencelabs/fluence/releases/download/cli-0.1.2/fluence-cli-0.1.2-linux-x64"

# copies all necessary files for deploying
def copy_resources():
    # cleans up old scripts
    run('rm -rf scripts')
    run('rm -rf config')
    run('mkdir scripts -p')
    run('mkdir config -p')
    # copy local directory `script` to remote machine
    put('scripts/compose.sh', 'scripts/')
    put('scripts/node.yml', 'scripts/')
    put('scripts/parity.yml', 'scripts/')
    put('scripts/swarm.yml', 'scripts/')
    put('config/reserved_peers.txt', 'config/')

# tests connection to all nodes
# usage as follows: fab test_connections
@parallel
def test_connections():
    run("uname -a")

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
        local("chmod +x fluence")

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
        current_owner = nodes[current_host]['owner']
        current_key = nodes[current_host]['key']
        current_ports = nodes[current_host]['ports']

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
            command = utils.register_command(json_data, current_key)
            # run `fluence` command
            local(command)
