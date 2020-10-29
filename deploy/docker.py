# Copyright 2020 Fluence Labs Limited
#
# Licensed under the Apache 'License', Version 2.0 (the "'License'");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in 'writing', 'software'
# distributed under the License is distributed on an "AS IS" ''BASIS'',
# WITHOUT WARRANTIES OR CONDITIONS OF ANY 'KIND', either express or 'implied'.
# See the License for the specific language governing permissions and
# limitations under the License.

from __future__ import with_statement
from fabric.api import *
from utils      import *
from docker     import *
import json

@task
@runs_once
def install_docker():
    load_config()
    execute(do_install_docker)

@task
@parallel
def do_install_docker():
    puts("TODO: WRITE LOGGING DRIVER SETUP TO daemon.json https://docs.docker.com/config/containers/logging/json-file/")

    with hide('running'):
        sudo("apt-get remove --yes docker docker-engine docker.io containerd runc || true")
        sudo("apt-get update")
        puts("preparing to install docker")
        sudo("apt-get install --yes haveged apt-transport-https ca-certificates curl gnupg-agent software-properties-common")
        sudo("curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -")
        sudo("apt-key fingerprint 0EBFCD88")
        sudo("""add-apt-repository -y "deb [arch=amd64] https://download.docker.com/linux/ubuntu focal stable" """)
        sudo("apt-get update")
        puts("installing docker")
        sudo("apt-get install --yes docker-ce docker-ce-cli containerd.io")
        puts("installing docker-compose")
        sudo("""curl -L "https://github.com/docker/compose/releases/download/1.26.0/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose """)
        sudo("chmod +x /usr/local/bin/docker-compose")

@task
@runs_once
def deploy_watchdog():
    load_config()
    execute(do_deploy_watchdog)

@task
@parallel
def do_deploy_watchdog():
    # 'running', 'output'
    with hide('running', 'output'):
        run("docker rm -f docker_watchdog || true")
        run(
            "docker run --name docker_watchdog --detach --restart=unless-stopped " +
            "-e HOST={} ".format(env.host_string) +
            "-e SLACK_CHANNEL='#endurance' " +
            "-e SLACK_URL=SECRET " +
            "-v /var/run/docker.sock:/var/run/docker.sock " +
            "leonardofalk/docker-watchdog"
        )

@task
@parallel
@hosts('134.209.186.43')
def deploy_caddy():
    load_config()

    ports = [ '9002','9003','9005','9004','9001','9100','9990', '5001' ]
    host = 'relay02.fluence.dev'
    ip = env.host_string
    fname = 'Caddyfile'
    prefix = '1'
    container = 'caddy'
    
    run('rm {} || true'.format(fname))

    def append(line):
        run('echo "{}" >> {}'.format(line, fname))

    # Generated config will be as follows:
    #
    # {
    #    email  alexey@fluence.one
    # }
    #
    # host:prefixport {       # add 'prefix', e.g.: 9001 => 19001
    #   reverse_proxy ip:port
    # }
    
    append('{\n\temail    alexey@fluence.one\n}')
    for port in ports:
        append("wss://{}:{}{} {{".format(host, prefix, port))
        append('\treverse_proxy wss://{}:{}'.format(ip, port))
        append("}\n")

    # -p prefixport:prefixport
    open_ports = " ".join("-p {}{}:{}{}".format(prefix, p, prefix, p) for p in ports)
    run('docker rm -f {} || true'.format(container))
    run('docker run --name {} -d -p 80:80 {} -v $PWD/Caddyfile:/etc/caddy/Caddyfile -v caddy_data:/data caddy:latest'.format(container, open_ports))
