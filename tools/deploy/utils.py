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

from __future__   import with_statement
from fabric.api   import *
from fabric.utils import *
import json
import time

# creates register command to register deployed node
def register_command(data, secret_key):
    command = "./fluence register \
        --node_ip            %s \
        --tendermint_key     %s \
        --tendermint_node_id %s \
        --contract_address   %s \
        --account            %s \
        --secret_key         %s \
        --api_port           %s \
        --capacity           %s \
        --eth_url            %s \
        --wait_syncing \
        --gas_price 10 \
        --base64_tendermint_key" % (
            data['node_ip'],
            data['tendermint_key'],
            data['tendermint_node_id'],
            data['contract_address'],
            data['account'],
            secret_key,
            data['api_port'],
            data['capacity'],
            data['ethereum_address']
        )

    return ' '.join(command.split())

def ensure_docker_group(user):
    from fabric.api import run

    run("groupadd docker &>/dev/null || true")
    run("usermod -aG docker %s || true" % user)

def chown_docker_sock(user):
    from fabric.api import run

    run("chmod a+r /var/run/docker.sock")
    run("chown %s:docker /var/run/docker.sock" % user)

def get_docker_pgid():
    from fabric.api import run

    output = run("grep docker /etc/group | cut -d ':' -f 3")
    output = output.stdout.splitlines()
    assert len(output) == 1
    return output[0]

RELEASE = "https://github.com/fluencelabs/fluence/releases/download/v0.1.6/fluence-cli-0.1.6-linux-x64"
def download_cli():
    # check if `fluence` file is exists
    result = local("[ -s fluence ] && echo 1 || echo 0", capture=True)
    if result == '0':
        print '`fluence` CLI file does not exist. Downloading it from ' + RELEASE
        local("wget " + RELEASE + " -O fluence")
        local("chmod +x fluence")

def get_tm_node_id():
    with hide('output'):
        out = run('docker run --user 0 --rm -v $HOME/.fluence/:/master -e TMHOME=/master/tendermint tendermint/tendermint show_node_id').stdout
        last_line = out.splitlines()[-1]
        return last_line

def get_tm_validator():
    with hide('output'):
        out = run('docker run --user 0 --rm -v $HOME/.fluence/:/master -e TMHOME=/master/tendermint tendermint/tendermint show_validator').stdout
        last_line = out.splitlines()[-1]
        return json.loads(last_line)['value']

def wait_node_started(api_port):
    echo("waiting for node to start ",prefix=True)
    while run('curl localhost:%s/' % api_port, quiet=True).failed:
        time.sleep(0.5)
        echo(".")
    echo('\n')

def echo(s,prefix=False):
    puts(s, end='', flush=True, show_prefix=prefix)

def register_node(current_host, current_key, ethereum_ip, contract_address, current_owner, api_port, capacity):
    tm_node_id = get_tm_node_id()
    tm_validator = get_tm_validator()
    data = {
        "node_ip": current_host,
        "ethereum_address": "http://" + ethereum_ip + ":8545",
        "tendermint_key": tm_validator,
        "tendermint_node_id": tm_node_id,
        "contract_address": contract_address,
        "account": current_owner,
        "api_port": api_port,
        "capacity": capacity,
    }
    command = register_command(data, current_key)
    with show('running'):
        # run `fluence` command
        local(command)

def get_config(environment):
    file = open("deployment_config.json", "r")
    info_json = file.read().rstrip()
    file.close()
    return json.loads(info_json)[environment]

def get_ipfs_address(config):
    if config.get('ipfs') is None:
        # Node and IPFS are connected via 'decentralized_storage_network' network, see node.yml & ipfs.yml
        return "http://ipfs:5001"
    else:
        return config['ipfs']

def get_image_tag(env):
    if not hasattr(env, 'image_tag'):
        return "v0.3.0"
    else:
        return env.image_tag

# copies all necessary files for deploying
def copy_resources():
    puts("Copying deployment files to node")
    # cleans up old scripts
    run('rm -rf scripts')
    run('mkdir scripts -p')
    run('mkdir scripts/functions -p')
    # copy local directory `script` to remote machine
    put('scripts/deploy.sh', 'scripts/')
    put('scripts/node.yml', 'scripts/')
    put('scripts/functions/asserts.sh', 'scripts/functions/')
    put('scripts/functions/docker.sh', 'scripts/functions/')

def home_dir():
    with hide('output'):
        return run('echo $HOME').stdout
