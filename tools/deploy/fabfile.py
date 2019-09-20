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
from utils      import *
import json

# Set the username
env.user = "fluence"
# Set to False to disable `[ip.ad.dre.ss] out:` prefix
env.output_prefix = True

config = get_config(env.environment)

if not env.hosts:
    # use addresses from config as fabric hosts
    env.hosts = config['nodes'].keys()
else:
    puts("will use hosts: %s" % env.hosts)

@task
@parallel
def deploy():
    with hide('running'):
        host        = env.host_string
        node        = config['nodes'][host]
        key         = node['key']
        owner       = node['owner']
        api_port    = node['api_port']
        capacity    = node['capacity']
        contract    = config['contract']
        ethereum_ip = config.get('ethereum_ip')
        ipfs        = get_ipfs_address(config)
        image_tag   = get_image_tag(env)
        storage_dir = home_dir() + '/.fluence/'
        chain       = 'rinkeby'

        download_cli()
        copy_resources()

        with cd("scripts"),\
             shell_env(CHAIN            = chain,
                       CONTRACT_ADDRESS = contract,
                       OWNER_ADDRESS    = owner,
                       API_PORT         = api_port,
                       HOST_IP          = host,
                       IPFS_ADDRESS     = ipfs,
                       ETHEREUM_IP      = ethereum_ip,
                       IMAGE_TAG        = image_tag,
                       FLUENCE_STORAGE  = storage_dir):
            run('chmod +x deploy.sh')
            run('./deploy.sh')
            wait_node_started(api_port)
            register_node(host, key, ethereum_ip, contract, owner, api_port, capacity)

@task
@parallel
def install_docker():
    with hide('running', 'output'):
        sudo("apt-get remove --yes docker docker-engine docker.io containerd runc || true")
        puts("apt-get update")
        sudo("apt-get update")
        puts("preparing to install docker")
        sudo("apt-get install --yes apt-transport-https ca-certificates curl gnupg-agent software-properties-common")
        sudo("curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -")
        sudo("apt-key fingerprint 0EBFCD88")
        sudo("""add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" """)
        sudo("apt-get update")
        puts("installing docker")
        sudo("apt-get install --yes docker-ce docker-ce-cli containerd.io")
        puts("installing docker-compose")
        sudo("""curl -L "https://github.com/docker/compose/releases/download/1.24.0/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose """)
        sudo("chmod +x /usr/local/bin/docker-compose")

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
        puts("IPFS: deploying...")
        results = execute(do_deploy_ipfs)
        puts("IPFS: deployed")
        puts("IPFS: interconnecting nodes...")
        external_addresses = [
            "/dns4/ipfs1.fluence.one/tcp/1036/ipfs/QmQodFqzJgqHyRDEG4abmMgHEV59AgXJ8foBeKgkazchNL",
            "/dns4/ipfs2.fluence.one/tcp/4001/ipfs/QmT2XFSBkLHPBFyae3o716Hs3qZidFhQrBHvfrMpZwgX7R"
        ]
        for ip, addrs in results.items():
            # filtering external addresses
            external_addresses += list(addr for addr in addrs if ip in addr)

        env.ipfs_addresses = external_addresses

        execute(connect_ipfs_nodes)
        puts("IPFS: bootstrap nodes added")

# usage: fab --set environment=stage,caddy_login=LOGIN,caddy_password=PASSWORD,role=slave deploy_netdata
@task
@parallel
def deploy_netdata():
    from fabric.contrib.files import upload_template

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
        env.home_dir = home_dir()
        upload_template("scripts/netdata/netdata.yml", "~/netdata/scripts/netdata.yml", context=env)
        if env.role == 'master':
            upload_template("scripts/netdata/netdata_caddy.yml", "~/netdata/scripts/netdata_caddy.yml", context=env)
            upload_template("config/netdata/Caddyfile", "~/netdata/config/Caddyfile", context=env)

        if env.role == 'slave':
            puts("netdata mode = slave")
            put("config/netdata/netdata_slave.conf", "~/netdata/config/netdata.conf")
            put("config/netdata/stream_slave.conf", "~/netdata/config/stream.conf")
        else:
            puts("netdata mode = master")
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
def test_connection():
    run("uname -a")

@task
@parallel
def add_fluence_user():
    # Connect to ssh with root@
    env.user = "root"
    user = "fluence"
    puts("Creating user %s" % user)
    run("adduser --gecos "" -q --disabled-password %s || true" % user)
    run("groupadd docker")
    run("usermod -a -G docker,sudo %s" % user)

    puts("Enabling paswordless sudoers for user %s" % user)
    run("echo '%s ALL=NOPASSWD: ALL' > /etc/sudoers.d/100-fluence" % user)

    puts("Copying ssh keys from /root/.ssh")
    run("mkdir /home/%s/.ssh/" % user)
    run("cp /root/.ssh/authorized_keys /home/%s/.ssh/authorized_keys" % user)
    run("chown -R %s:%s /home/%s/.ssh/" % (user, user, user))
    run("chmod 700 /home/%s/.ssh" % user)
    run("chmod 600 /home/%s/.ssh/authorized_keys" % user)

@task
@parallel
def disable_root_ssh():
    with hide('output', 'running'):
        puts("Disabling ssh root login")

        # Check whether config already contains PermitRootLogin setting
        contains = run("grep -Pq '^PermitRootLogin[ \t]+' /etc/ssh/sshd_config").succeeded
        if contains:
            # Replace PermitRootLogin => PermitRootLogin no
            sudo("sed -i '/^PermitRootLogin[ \\t]\+\w\+$/{ s//PermitRootLogin no/g; }' /etc/ssh/sshd_config",warn_only=True)
        else:
            # Insert 'PermitRootLogin no'
            sudo("echo 'PermitRootLogin no' >> /etc/ssh/sshd_config")
        sudo("service ssh restart")
