from __future__             import with_statement
from collections            import namedtuple
from fabric.api             import *
from fabric.contrib.files   import append
from utils                  import *
from compose                import *
from collections            import namedtuple
from time                   import sleep


# DTOs
Node = namedtuple("Node", "peer_id tcp ws")
Service = namedtuple("Service", "port multiaddr")

FLUENCE_NODE_PORT = "7777"
FLUENCE_CLIENT_PORT = "9999"
PEER_ID_MARKER = "server peer id"


@task
@runs_once
def deploy_fluence():
    # 'running', 'output'
    with hide():
        load_config()
        target = target_environment()
        env.hosts = target["bootstrap"]

        puts("Fluence: deploying bootstrap")
        results = execute(deploy_bootstrap)
        bootstraps = fill_addresses(results.items())
        bootstrap = bootstraps[0]
        env.bootstraps = map(lambda b: b.tcp.multiaddr, bootstraps)


        env.hosts = target["hosts"]
        puts("Fluence: deploying rest of the nodes")
        results = execute(deploy_nodes)
        nodes = fill_addresses(results.items())

        puts("Fluence: deployed.\nAddresses:\n%s" % "\n".join(
            "{} {} {}".format(n.tcp.multiaddr, n.ws.multiaddr, n.peer_id) for n in nodes))
        puts("Bootstrap:\n%s" % "\n".join(
            "{} {} {}".format(n.tcp.multiaddr, n.ws.multiaddr, n.peer_id) for n in bootstraps))


@task
@parallel
def deploy_bootstrap():
    target = target_environment()
    yml = "fluence_bootstrap.yml"
    keypair = get_keypairs(yml, get_host_idx(containers=1), count=1)
    gen_compose_file(
        out=yml,
        container_tag=target['container_tag'],
        scale=1,
        is_bootstrap=True,
        bootstraps=target['external_bootstraps'],
        host=env.host_string,
        management_key=target['management_key'],
        keypairs=keypair,
    )

    return do_deploy_fluence(yml)


@task
@parallel
def deploy_nodes():
    target = target_environment()

    yml = "fluence.yml"
    scale = target["containers_per_host"]
    keypairs = get_keypairs(yml, get_host_idx(scale), count=scale)
    gen_compose_file(
        out=yml,
        container_tag=target['container_tag'],
        scale=scale,
        is_bootstrap=False,
        bootstraps=env.bootstraps + target['external_bootstraps'],
        host=env.host_string,
        management_key=target['management_key'],
        keypairs=keypairs,
    )

    return do_deploy_fluence(yml)


@task
@parallel
# returns {ip: Node}
def do_deploy_fluence(yml="fluence.yml"):
    with hide():
        compose("pull", yml)
        compose('rm -fs', yml)
        compose('up --no-start', yml)  # was: 'create'
        copy_configs(yml)
        compose("restart", yml)
        sleep(5)
        addrs = get_fluence_addresses(yml)
        return addrs


def get_host_idx(containers):
    return env.hosts.index(env.host_string) * containers

def copy_configs(yml):
    # there's no `cp` in `docker-compose`: https://github.com/docker/compose/issues/5523
    put("Config.toml", "./")
    containers = compose('ps -q', yml).splitlines()
    for id in containers:
        run('docker cp ./Config.toml %s:/Config.toml' % id)

# returns [Node]
def get_fluence_addresses(yml="fluence.yml"):
    containers = compose('ps -q', yml).splitlines()
    nodes = []
    for id in containers:
        (tcp_port, ws_port) = get_ports(id)
        peer_id = get_fluence_peer_ids(id)
        node = Node(peer_id=peer_id, tcp=Service(tcp_port, None), ws=Service(ws_port, None))
        nodes.append(node)
    return nodes

# Assuming Fluence's tcp port starts with 7
# and websocket port starts with 9
def is_fluence_port(host_port):
    is_tcp = '0.0.0.0:7' in host_port
    is_ws = '0.0.0.0:9' in host_port
    return is_tcp or is_ws

# returns (tcp port, ws port)
def get_ports(container):
    from itertools import chain
    lines = run('docker port %s' % container).splitlines()
    ports = chain.from_iterable(l.split('/tcp -> ') for l in lines)
    # filter by host port and remove 0.0.0.0 part
    ports = list(port.replace('0.0.0.0:', '') for port in ports if is_fluence_port(port))
    (a, b) = ports
    # tcp port starts with 7
    if a.startswith('7'):
        return (a, b)
    else:
        return (b, a)


def get_fluence_peer_ids(container, yml="fluence.yml"):
    logs = run('docker logs --tail 10000 %s' % container).splitlines()
    return parse_peer_ids(logs)


# returns (node_peer_id, peer_peer_id)
def parse_peer_ids(logs):
    def after_eq(line):
        return line.split("=")[-1].strip()

    peer_id = None
    for line in logs:
        if PEER_ID_MARKER in line:
            peer_id = after_eq(line)
    return peer_id


def compose(cmd, yml="fluence.yml"):
    return run('docker-compose -f %s %s' % (yml, cmd))


def service(yml):
    return yml.replace(".yml", "")


# takes: dict {ip: Node}
# returns: [Node]
def fill_addresses(nodes_dict):
    result = []
    for ip, nodes in nodes_dict:
        for node in nodes:
            # node service multiaddr
            node = node._replace(tcp=fill_multiaddr(ip, node.tcp))
            # peer service multiaddr
            node = node._replace(ws=fill_multiaddr(ip, node.ws, suffix="/ws"))
            result.append(node)
    return result


def fill_multiaddr(ip, service, suffix=""):
    return service._replace(multiaddr="/ip4/{}/tcp/{}{}".format(ip, service.port, suffix))
