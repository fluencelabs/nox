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

# owners and private keys for specific ip addresses
# todo get this info from some sources
# TO USE: replace values inside <> with your actual values
info = {'46.101.213.180': {'owner': '0x6f0773cd2965c014d8a9d6ddd8e739568edeb7f5', 'key': '906ccd15743cf56dfe1d08c7ca5333560ecc88675da514a517ddeed2dc2cb820', "ports": "25000:25099"},
       '207.154.240.52': {'owner': '0x1f111701508d76b27ff59d22e839cc907f273031', 'key': '3b00a2b6bbfc0cf1c4b041b8b6bad8e4bd718d88e7a9496a5dc4ca885b17d097', "ports": "25000:25099"},
       '159.89.17.35': {'owner': '0x5903730e872fb2b0cd4402c69d6d43c86e973db7', 'key': '0x52c685b72c548da46ee7e595b4003635a1dab3f281dc26b9a13a5b0ea736d3b1', "ports": "25000:25099"},
       '207.154.232.92': {'owner': '0x8965c5c7e89641dc44d9798dc21fc8294c68b80e', 'key': '0x0ba8dbe75adafab30aae7e276e8a977b37d2cac025968b2cd7c50d1eaf2c4a12', "ports": "25000:25099"},
       '142.93.32.156': {'owner': '0x2bd8cf9c906f12aeeed95b262b431bfcad33cb52', 'key': '0xc8b8c436470be4777478124abfd2f908a5c44b796f4955a159795626f36648e9', "ports": "25000:25099"},
       '209.97.191.196': {'owner': '0x444074049976675e7289da3dd08d0c01394db5ad', 'key': '0xa2b08211cc8f2ed6f4e5e4bb00a569dc32765bec38be4186ad6ee16a80670d4e', "ports": "25000:25099"},
       '178.62.77.192': {'owner': '0x7DdAE2d6118562AaC405284bb297C9A53d975326', 'key': '0x3639CC2D3D27ABF76509077EFC0BE6093290F0F8739C00BDDA6504B9D9FC66C2', "ports": "25000:25099"}}

RELEASE_LINUX="https://github.com/fluencelabs/fluence/releases/download/cli-0.1.2/fluence-cli-0.1.2-linux-x64" #"https://github.com/fluencelabs/fluence/releases/download/untagged-3f7e10bd802b3149036d/fluence-linux-x64"
RELEASE_MACOS="https://github.com/fluencelabs/fluence/releases/download/cli-0.1.2/fluence-cli-0.1.2-mac-x64" #"https://github.com/fluencelabs/fluence/releases/download/untagged-3f7e10bd802b3149036d/fluence-linux-x64"

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


# comment this annotation to deploy sequentially
@parallel
def deploy():

    # check if `fluence` file is exists
    result = local("[ -s fluence ] && echo 1 || echo 0", capture=True)
    if (result == '0'):
        print
        # todo: add correct link to CLI
        print '`fluence` CLI file does not exist. Downloading it from ' + RELEASE
        local("wget -O fluence " + RELEASE)
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
            command = utils.register_command(json_data, current_key)
            # run `fluence` command
            local(command)

