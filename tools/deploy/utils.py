from __future__ import with_statement
from fabric.api import *
import json

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
    return run('docker run --user 0 --rm -v $HOME/.fluence/:/master -e TMHOME=/master/tendermint tendermint/tendermint show_node_id')

def get_tm_validator():
    out = run('docker run --user 0 --rm -v $HOME/.fluence/:/master -e TMHOME=/master/tendermint tendermint/tendermint show_validator')
    return json.loads(out)['value']

def register_node(current_host,
                  current_key,
                  ethereum_ip,
                  contract_address,
                  current_owner,
                  api_port,
                  capacity):
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
    info = json.loads(info_json)[environment]