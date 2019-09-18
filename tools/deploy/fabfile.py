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

from __future__ import with_statement
from fabric.api import *
import json
import utils

if hasattr(env, 'environment'):
    environment = env.environment

    # gets deployed contract address from a file
    file = open("deployment_config.json", "r")
    info_json = file.read().rstrip()
    file.close()

    info = json.loads(info_json)[environment]
    contract = info['contract']
    nodes = info['nodes']
    env.swarm = info.get('swarm')
    env.ipfs = info.get('ipfs')
    env.ethereum_ip = info.get('ethereum_ip')
else:
    # gets deployed contract address from a file
    file = open("instances.json", "r")
    nodes_json = file.read().rstrip()
    file.close()

    file = open("scripts/contract.txt", "r")
    contract = file.read().rstrip()
    file.close()

    nodes = json.loads(nodes_json)

if not env.hosts:
    env.hosts = nodes.keys()
else:
    print "will use hosts: %s" % env.hosts

# Set the username
env.user = "root"

# Set to False to disable `[ip.ad.dre.ss] out:` prefix
env.output_prefix = True

# Linux
RELEASE = "https://github.com/fluencelabs/fluence/releases/download/v0.1.6/fluence-cli-0.1.6-linux-x64"

# macOS
# RELEASE = "https://github.com/fluencelabs/fluence/releases/download/v0.1.6/fluence-cli-0.1.6-mac-x64"


# copies all necessary files for deploying
def copy_resources():
    print "Copying deployment files to node"
    # cleans up old scripts
    run('rm -rf scripts')
    run('rm -rf config')
    run('mkdir scripts -p')
    run('mkdir config -p')
    # copy local directory `script` to remote machine
    put('scripts/compose.sh', 'scripts/')
    put('scripts/node.yml', 'scripts/')
    put('scripts/parity.yml', 'scripts/')
    put('scripts/geth.yml', 'scripts/')
    put('scripts/swarm.yml', 'scripts/')
    put('scripts/ipfs.yml', 'scripts/')
    put('config/reserved_peers.txt', 'config/')


# tests connection to all nodes
# usage as follows: fab test_connections
@task
@parallel
def test_connections():
    run("uname -a")


@task
@parallel
def deploy():
    with hide('running'):
        chain = 'rinkeby'

        contract_address = contract
        current_host     = env.host_string
        current_owner    = nodes[current_host]['owner']
        current_key      = nodes[current_host]['key']
        api_port         = nodes[current_host]['api_port']
        capacity         = nodes[current_host]['capacity']
        ipfs             = get_ipfs_address()
        ethereum_ip      = env.ethereum_ip
        image_tag        = get_image_tag()

        download_cli()
        copy_resources()

        with cd("scripts"):
            with shell_env(CHAIN=chain,
                           CONTRACT_ADDRESS=contract_address,
                           OWNER_ADDRESS=current_owner,
                           API_PORT=api_port,
                           HOST_IP=current_host,
                           IPFS_ADDRESS=ipfs,
                           ETHEREUM_IP=ethereum_ip,
                           IMAGE_TAG=image_tag):
                run('chmod +x compose.sh')
                run('./compose.sh')
                register_node(current_host,ethereum_ip,tm_validator,tm_node_id,contract_address,current_owner,api_port,capacity)

def get_ipfs_address():
    if env.ipfs is None:
        # Node and IPFS are connected via 'decentralized_storage_network' network, see node.yml & ipfs.yml
        return "http://ipfs:5001"
    else:
        return env.ipfs

def get_image_tag():
    if not hasattr(env, 'image_tag'):
        return "v0.3.0"
    else:
        return env.image_tag

# usage: fab --set environment=stage,caddy_login=LOGIN,caddy_password=PASSWORD,role=slave deploy_netdata
@task
@parallel
def deploy_netdata():
    from fabric.contrib.files import upload_template
    from utils import ensure_docker_group, chown_docker_sock, get_docker_pgid

    if not hasattr(env, 'caddy_port'):
        env.caddy_port = 1337  # set default port

    usage = "usage: fab --set caddy_login=LOGIN,caddy_password=PASSWORD,caddy_port=1337 deploy_netdata"
    assert hasattr(env, 'caddy_login'), usage
    assert hasattr(env, 'caddy_password'), usage

    if not hasattr(env, 'role'):
        env.role = 'slave'

    with hide('running', 'output'):
        if env.role == 'master':
            run("docker pull abiosoft/caddy")
        run("docker pull netdata/netdata")
        run("mkdir -p ~/netdata/scripts")
        run("mkdir -p ~/netdata/config")
        run("mkdir -p ~/.local/netdata_cache")
        run("chmod o+rw ~/.local/netdata_cache")
        env.home_dir = run("pwd").stdout
        upload_template("scripts/netdata/netdata.yml", "~/netdata/scripts/netdata.yml", context=env)
        if env.role == 'master':
            upload_template("scripts/netdata/netdata_caddy.yml", "~/netdata/scripts/netdata_caddy.yml", context=env)
            upload_template("config/netdata/Caddyfile", "~/netdata/config/Caddyfile", context=env)

        if env.role == 'slave':
            print "netdata mode = slave"
            put("config/netdata/netdata_slave.conf", "~/netdata/config/netdata.conf")
            put("config/netdata/stream_slave.conf", "~/netdata/config/stream.conf")
        else:
            print "netdata mode = master"
            put("config/netdata/netdata_master.conf", "~/netdata/config/netdata.conf")
            put("config/netdata/stream_master.conf", "~/netdata/config/stream.conf")

        ensure_docker_group(env.user)
        chown_docker_sock(env.user)
        pgid = get_docker_pgid()

        with shell_env(COMPOSE_IGNORE_ORPHANS="true"):
            with show('running'):
                if env.role == 'slave':
                    run("PGID=%s HOSTNAME=$HOSTNAME docker-compose --compatibility -f ~/netdata/scripts/netdata.yml up -d" % pgid)
                else:
                    run("PGID=%s HOSTNAME=$HOSTNAME docker-compose --compatibility -f ~/netdata/scripts/netdata_caddy.yml -f ~/netdata/scripts/netdata.yml up -d" % pgid)

@task
@parallel
def install_docker():
    with hide('running', 'output'):
        run("apt-get remove --yes docker docker-engine docker.io containerd runc || true")
        print "apt-get update"
        run("apt-get update")
        print "preparing to install docker"
        run("apt-get install --yes apt-transport-https ca-certificates curl gnupg-agent software-properties-common")
        run("curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -")
        run("apt-key fingerprint 0EBFCD88")
        run("""sudo add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" """)
        run("apt-get update")
        print "installing docker"
        run("apt-get install --yes docker-ce docker-ce-cli containerd.io")
        print "installing docker-compose"
        run("""curl -L "https://github.com/docker/compose/releases/download/1.24.0/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose """)
        run("chmod +x /usr/local/bin/docker-compose")

@task
@parallel
def do_deploy_ipfs():
    with hide('running', 'output'):
        put('ipfs/ipfs.yml', './')
        run('docker-compose -f ./ipfs.yml up -d')
        output = run('docker-compose -f ./ipfs.yml exec ipfs ipfs id')
        ipfs_addresses = json.loads(output)['Addresses']
        return ipfs_addresses

@task
@parallel
def connect_ipfs_nodes():
    with hide('running', 'output'):
        for addr in env.ipfs_addresses:
            run('docker-compose -f ./ipfs.yml exec ipfs ipfs bootstrap add %s' % addr)

@task
@runs_once
# example: fab --set environment=stage deploy_ipfs
def deploy_ipfs():
    with hide('running', 'output'):
        print "IPFS: deploying..."
        results = execute(do_deploy_ipfs)
        print "IPFS: deployed"
        print "IPFS: interconnecting nodes..."
        external_addresses = [
            "/dns4/ipfs1.fluence.one/tcp/1036/ipfs/QmQodFqzJgqHyRDEG4abmMgHEV59AgXJ8foBeKgkazchNL",
            "/dns4/ipfs2.fluence.one/tcp/4001/ipfs/QmT2XFSBkLHPBFyae3o716Hs3qZidFhQrBHvfrMpZwgX7R"
        ]
        for ip, addrs in results.items():
            # filtering external addresses
            external_addresses += list(addr for addr in addrs if ip in addr)

        env.ipfs_addresses = external_addresses

        execute(connect_ipfs_nodes)
        print "IPFS: bootstrap nodes added"
