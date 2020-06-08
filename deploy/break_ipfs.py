from __future__ import with_statement
from utils      import load_config
from fabric.api import *
from ipfs       import *
from time       import sleep

IMAGE="folexflu/ipfs:noexposure"
NET_LBL="break_ipfs"

def run_ipfs(name, p2p_port, api_port, network):
    if not api_port is None and not p2p_port is None:
        # contract, required for get_ipfs_nodes to calculate ports correctly
        assert str(api_port).startswith("5"), "API port must start with 5"
        assert str(p2p_port).startswith("4"), "API port must start with 4"

    with hide('running', 'output'):
        cmd = 'docker run -d --name {}'.format(name)
        if not network is None:
            cmd += " --network {}".format(network)
        if not p2p_port is None:
            cmd += " -p {}:{}".format(p2p_port, p2p_port)
        if not api_port is None:
            cmd += " -p {}:{}".format(api_port, api_port)
        cmd += " " + IMAGE
        cmd += " daemon --migrate=true"
        container = run(cmd).splitlines()[-1]
        return container

def configure_ipfs(containers, open_ports, bootstraps=[]):
    with hide('running', 'output'):
        nodes = get_ipfs_nodes(containers)
        clear_bootstrap_nodes(nodes)
        put_swarm_key(nodes)
        if open_ports: 
            set_ports(nodes)
            announce_host(nodes)
        if bootstraps: 
            add_bootstraps(nodes, bootstraps)
        restart(nodes)
        return nodes

def announce_host(nodes):
    for node in nodes:
        address = "/ip4/{}/tcp/{}".format(env.host, node.p2p_port)
        run('docker exec {} ipfs config --json Addresses.Announce \'["{}"]\''.format(node.container_id, address))

def add_bootstraps(nodes, bootstraps):
    with hide('running', 'output'):
        for node in nodes:
            bs = " ".join(bootstraps)
            run('docker exec {} ipfs bootstrap add {}'.format(node.container_id, bs))

def create_network(name, subnet):
    with hide('running', 'output'):
        run('docker network create --label={} {} --subnet=172.19.{}.0/24'.format(NET_LBL, name, subnet))
        return name

def clear_networks():
    networks = run('docker network ls -qf "label={}"'.format(NET_LBL)).splitlines()
    if networks:
        networks = " ".join(networks)
        run('docker network rm {}'.format(networks))

def status(container):
    return run('docker ps -f "id = {}" --format "{{{{.Status}}}}"'.format(container))

def restart(nodes):
    for node in nodes:
        run('docker restart {}'.format(node.container_id))

def get_containers():
    return run('docker ps -aq').splitlines()

# prefix controls hundreds in the port number. i.e., prefix = 7, api port will be 57XX
@task
@parallel
def run_ipfs_containers(name, prefix = 1, count = 1, open_ports = True, bootstraps = []):
    assert isinstance(prefix, (int, long)), "prefix must be a number" # oh I love python
    containers = []
    for i in range(1, count + 1):
        cont_name = "{}_{}_{}".format(name, prefix, i)
        if open_ports:
            api_port = "5{}{:02}".format(prefix, i)
            p2p_port = "4{}{:02}".format(prefix, i)
            network = None
        else:
            api_port = None
            p2p_port = None    
            network = create_network(cont_name, subnet=prefix + i)

        container = run_ipfs(cont_name, api_port = api_port, p2p_port = p2p_port, network = network)
        containers.append(container)
    # sleep(1)
    nodes = configure_ipfs(containers, open_ports, bootstraps)
    return nodes

@task
@parallel
def remove_all_containers():
    with hide('running', 'output'):
        containers = get_containers()
        for id in containers:
            run('docker rm -f {}'.format(id))
        clear_networks()

@task
def publish_file(provider):
    content = time.strftime("%a, %d %b %Y %H:%M:%S", time.localtime())
    with hide('running', 'output'):
        run('echo "{}" > file'.format(content))
        run('docker cp file {}:/file'.format(provider.container_id))
        return run('docker exec {} ipfs add -Q /file'.format(provider.container_id))

@task
def evict_good_node(bad):
    containers = bad[env.host]
    with hide('running', 'output'):
        restart(containers)

@task
def download_file(client, file_hash):
    run('docker exec {} ipfs dht query -v {}'.format(client.container_id, file_hash))
    return run('docker exec {} ipfs get {}'.format(client.container_id, file_hash))

@task
@runs_once
def deploy_broken_ipfs():
    load_config()
    
    ### Remove all containers on all hosts
    env.hosts = env.config['ipfs']['bootstrap'] + env.config['ipfs']['good'] + env.config['ipfs']['provider'] + env.config['ipfs']['bad_nodes']
    puts("Removing all containers & networks everywhere")
    with hide('running'):
        execute(remove_all_containers)

    ### Deploy bootstrap
    env.hosts = env.config['ipfs']['bootstrap']
    puts("Deploying bootstrap")
    with hide('running'):
        bootstrap = execute(run_ipfs_containers, name="bootstrap")
        puts("Bootstrap: {}".format(bootstrap))
        bootstrap = generate_addresses(bootstrap.items())

    ### Deploy good node
    env.hosts = env.config['ipfs']['good']
    puts("Deploying good node")
    with hide('running'):
        good = execute(run_ipfs_containers, name="good", prefix=2, bootstraps=bootstrap)
    puts("Good: {}".format(good))

    ### Deploy lots of bad nodes
    env.hosts = env.config['ipfs']['bad_nodes']
    puts("Deploying bad nodes")
    with hide('running'):
        bad = execute(
            run_ipfs_containers, 
            name="bad", prefix=1, count=50, open_ports=False, bootstraps=bootstrap
        )
    puts("Bad: {}".format(bad))

    ### Deploy provider node
    env.hosts = env.config['ipfs']['provider']
    puts("Deploying provider")
    with hide('running'):
        provider = execute(run_ipfs_containers, name="provider", prefix=3, bootstraps=bootstrap)
    puts("Provider: {}".format(provider))
    provider = provider.values()[0][0]

    puts("Uploading file")
    file_hash = execute(publish_file, provider).values()[0]
    puts("file hash: {}".format(file_hash))

    puts("Evicting good node")
    env.hosts = env.config['ipfs']['bad_nodes']
    execute(evict_good_node, bad)
    # check_good_node_evicted()

    answer = prompt("\nEvict?")
    while answer is "yes":
        execute(evict_good_node, bad)
        answer = prompt("Check it's really evicted. Evict again?")

    ### Deploy client node
    # will deploy on the same host as good
    env.hosts = env.config['ipfs']['good']
    puts("Deploying client")
    with hide('running'):
        client = execute(
            run_ipfs_containers, 
            name="client", prefix=4, open_ports=False, bootstraps=bootstrap
        )
    puts("Client: {}".format(client))

    client = client.values()[0][0]
    puts("Downloading file")
    execute(download_file, client, file_hash)

    # TODO: check good node was in bucket, but evicted
    # TODO: check good node is available from client

'''
def deploy_nodes(bootstrap, close_p2p, close_api, count = 1):
    infos = []
    for i in 1..count:
        docker('run -d ipfs')
        clear_bootstrap()
        copy_swarm_key()
        if bootstrap is not None:
            add_bootstrap(bootstrap)
        # TODO: what else?
        restart()
        infos.append ( get_ports_peer_id_etc() )
    if count == 1:
        return info[0] # ha-ha suck it, Haskell!
    else
        return infos

def deploy_bootstrap():
    deploy_node(None, False, False)

def deploy_bad_nodes(bootstrap, addresses):
    env.hosts = addresses
    results = execute(deploy_node, bootstrap, True, True)

def reconnect_bad_nodes(addresses):
    env.hosts = addresses
    execute(restart_containers)

def deploy_good_node(bootstrap): # P2P ports must be closed, API port should be open
    return deploy_node(bootstrap, True, True)
    # connect to bootstrap
    # 

def check_good_node_evicted():
    # TODO: check logs of the bootstrap nodes?
    #       there should be no sign of good node

def deploy_provider()
def publish_file()

def evict_good_node():
    reconnect_bad_nodes()
    evict_good_node(config.bad_nodes)
    check_good_node_evicted() # TODO: would be awesome to check periodically in background


def run():
    bootstrap = deploy_bootstrap(config.bootstrap)
    deploy_bad_nodes(bootstrap, config.bad_nodes)
    deploy_good_node(config.good_node)
    
    provider = deploy_provider(config.provider)
    publish_file(provider)

    evict_good_node()
    check_good_node_evicted()

    deploy_client_node()
    download_file() # should fail
'''