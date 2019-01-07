# Import Fabric's API module
from fabric.api import *

# owners and private keys for specific ip addresses
# todo get this info from some sources
# TO USE: replace values inside <> with your actual values
info = {'<ip1>': {'owner': '<eth address1>', 'key': '<private key1>'},
        '<ip2>': {'owner': '<eth address2>', 'key': '<private key2>'}}

# change this if you want to deploy on another hosts
# TO USE: replace values inside <> with your actual values
env.hosts = ['<ip1>', "<ip2>"]

# Set the username
env.user = "root"
 
 
def copy_resources():

    # copy local directory `script` to remote machine
    put('scripts', '.')

# comment this annotation to deploy sequentially
@parallel
def deploy():
 
    copy_resources()

    with cd("scripts"):

        # change for another chain
        # todo changing this variable should recreate parity container
        # todo support/implement contract deployment on 'dev' chain
        chain='kovan'

        # actual fluence contract address
        contract_address='0x8013D56197629180ABd69e8e24556054016B693f'

        # getting owner and private key from `info` dictionary
        current_host = env.host_string
        current_owner = info[current_host]['owner']
        current_key = info[current_host]['key']

        with shell_env(CHAIN=chain,
                       CONTRACT_ADDRESS=contract_address,
                       BZZ_KEY=current_owner,
                       OWNER_ADDRESS=current_owner,
                       PRIVATE_KEY=current_key):
            run('chmod +x compose.sh')
            run('chmod +x fluence')
            run('./compose.sh')
