from __future__ import with_statement
from fabric.api import *
from fabric.contrib.files import append
from utils import *
import json
from collections import namedtuple

IPFS_P2P_PORT = "4001"
IPFS_API_PORT = "5001"

# DTOs
IpfsNode = namedtuple("IpfsNode", "container_id p2p_port api_port peer_id")


@task
@runs_once
# example: fab deploy_ipfs
def deploy_ipfs():
    with hide('running', 'output'):
        load_config()

        puts("IPFS: deploying...")
        # deploying IPFS per node
        ipfs_nodes = execute(do_deploy_ipfs)
        puts("IPFS: deployed")
        puts("IPFS: interconnecting nodes... (%s bootstrap)" % len(env.config['ipfs']['bootstrap']))
        # start with existing bootstrap nodes
        external_addresses = env.config['ipfs']['bootstrap']
        external_addresses += generate_addresses(ipfs_nodes.items())

        puts("IPFS: addresses")
        print_api_addresses(ipfs_nodes.items())

        env.ipfs_addresses = external_addresses

        execute(connect_ipfs_nodes)
        puts("IPFS: done")

        # Uncomment if IPFS Cluster is needed
        # setup_ipfs_cluster()


# takes: dict {ip: [IpfsNode]}
# returns: [string]
def generate_addresses(ipfs_nodes):
    result = []
    for ip, nodes in ipfs_nodes:
        for node in nodes:
            multiaddr = "/ip4/{}/tcp/{}/ipfs/{}".format(ip, node.p2p_port, node.peer_id)
            result.append(multiaddr)
    return result


def print_api_addresses(ipfs_nodes):
    for ip, nodes in ipfs_nodes:
        for node in nodes:
            addr = "{}:{}".format(ip, node.api_port)
            puts(addr)


@task
@parallel
# returns {ip: [(port, peer_id)]}
def do_deploy_ipfs():
    with hide('running', 'output'):
        put('ipfs.yml', './')
        with shell_env(HOST=env.host_string):
            # TODO: move somewhere
            run('docker-compose -f ./ipfs.yml up -d')
            nodes = get_ipfs_nodes(containers=get_containers())
            # put_swarm_key(nodes)
            # clear_bootstrap_nodes(nodes)
            set_ports(nodes)
            # force_private_net()
            return nodes


def force_private_net():
    with shell_env(LIBP2P_FORCE_PNET="1"):
        run('docker-compose -f ./ipfs.yml restart')


# returns [IpfsNode]
def get_ipfs_nodes(containers):
    result = []
    for id in containers:
        (api_port, p2p_port) = get_ports(id)
        peer_id = get_ipfs_peer_id(id)
        result.append(IpfsNode(id, p2p_port, api_port, peer_id))
    return result


def get_containers():
    return run('docker-compose -f ./ipfs.yml ps -q').splitlines()


# Assuming API port always starts from 5, and P2P port starts from 4
# returns (node port, peer port)
def get_ports(container):
    from itertools import chain
    lines = run('docker port %s' % container).splitlines()
    if len(lines) == 0:
        return None, None

    ports = chain.from_iterable(l.split('/tcp -> ') for l in lines)
    # filter by host port and remove 0.0.0.0 part
    ports = list(port.replace('0.0.0.0:', '') for port in ports if '0.0.0.0' in port)
    (a, b) = ports
    # IPFS API port always start from 5, the other one will be P2P (swarm) port
    if a.startswith('5'):
        return a, b
    else:
        return b, a


def get_ipfs_peer_id(container):
    output = run('docker exec %s ipfs id' % container, warn_only=True, quiet=True)
    i = 0
    while output.failed and i < 10:
        i += 1
        output = run('docker exec %s ipfs id' % container, warn_only=True, quiet=True)
    if i >= 10:
        puts("ERROR while getting ipfs id")
        return 'PEER_ID_ERROR'
    try:
        peer_id = json.loads(output)['ID']
        return peer_id
    except:
        puts("ERROR while parsing json from %s" % output)
        return 'PEER_ID_ERROR'  # Now that's awesome


def put_swarm_key(nodes):
    put('swarm.key', 'swarm.key')
    for node in nodes:
        run('docker cp swarm.key %s:/data/ipfs/swarm.key' % node.container_id)


def clear_bootstrap_nodes(nodes):
    for node in nodes:
        run('docker exec %s ipfs bootstrap rm --all' % node.container_id)


def set_ports(nodes):
    for node in nodes:
        run('docker exec %s ipfs config Addresses.API /ip4/0.0.0.0/tcp/%s' % (node.container_id, node.api_port))
        run('docker exec %s ipfs config --json Addresses.Swarm \'["/ip4/0.0.0.0/tcp/%s"]\'' % (
            node.container_id, node.p2p_port))


@task
@parallel
def collect_ipfs_cluster_addresses():
    with hide('running', 'output'):
        output = run('docker-compose -f ./ipfs.yml exec ipfs-cluster ipfs-cluster-ctl id')
        lines = output.split('\n')
        cluster_addresses = filter(lambda x: '9096' in x and '127.0.0.1' in x, lines)
        cluster_addresses = map(lambda x: x.translate(None, '- ').replace("127.0.0.1", env.host_string),
                                cluster_addresses)
        return cluster_addresses


def setup_ipfs_cluster():
    puts("IPFS: bootstrapping cluster...")
    results = execute(collect_ipfs_cluster_addresses)
    env.cluster_addresses = []
    for ip, addrs in results.items():
        env.cluster_addresses += addrs

    execute(bootstrap_ipfs_cluster)


@task
@parallel
def bootstrap_ipfs_cluster():
    with hide('running', 'output'):
        PEERSTORE = '~/.ipfs/cluster/peerstore'
        run('rm %s' % PEERSTORE)
        run('touch %s' % PEERSTORE)
        run('chown 1000:users %s' % PEERSTORE)
        for addr in env.cluster_addresses:
            run("echo \"%s\" >> ~/.ipfs/cluster/peerstore" % addr)
        run('docker-compose -f ./ipfs.yml restart ipfs-cluster')


@task
@parallel
def connect_ipfs_nodes():
    with hide('running', 'output'):
        containers = get_containers()
        for id in containers:
            peer_id = get_ipfs_peer_id(id)
            # chunk into 50-chunks, that's near command-length limit of fabric or something like that
            chunked = chunks(filter(lambda a: peer_id not in a, env.ipfs_addresses), 50)
            for addrs in chunked:
                peers = ' '.join(addrs)
                run('docker exec %s ipfs bootstrap add %s' % (id, peers))
                run('docker exec %s ipfs swarm connect %s' % (id, peers))


def chunks(l, n):
    n = max(1, n)
    return (l[i:i + n] for i in xrange(0, len(l), n))
