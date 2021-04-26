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

def wait_node_started(api_port):
    echo("waiting for node to start ",prefix=True)
    while run('curl localhost:%s/' % api_port, quiet=True).failed:
        time.sleep(0.5)
        echo(".")
    echo('\n')

def echo(s,prefix=False):
    puts(s, end='', flush=True, show_prefix=prefix)

def get_config(environment):
    file = open("deployment_config.json", "r")
    info_json = file.read().rstrip()
    file.close()
    return json.loads(info_json)[environment]

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

def load_config():
    # Set to False to disable `[ip.ad.dre.ss] out:` prefix
    env.output_prefix = True

    cfg_file = open("new_config.json", "r")
    env.config = json.loads(cfg_file.read().rstrip())
    cfg_file.close()

    target = target_environment()
    # Set the username
    env.user = target.user

    if not env.hosts:
        # use addresses from config as fabric hosts
        env.hosts = target['hosts']
    else:
        puts("will use hosts: %s" % env.hosts)

def target_environment():
    return env.config.environments[env.target]

def docker_tag():
    return target_environment['docker_tag']

def get_keypair(yml, idx):
    return target_environment['keypairs'][yml][idx]