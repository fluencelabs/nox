import subprocess
import utils
import json

def test_json_format():
    env_with_args = dict()
    env_with_args["EXTERNAL_HOST_IP"] = "123"
    env_with_args["TENDERMINT_KEY"] = "456"
    env_with_args["TENDERMINT_NODE_ID"] = "qwe"
    env_with_args["CONTRACT_ADDRESS"] = "ca"
    env_with_args["OWNER_ADDRESS"] = "oa"
    env_with_args["START_PORT"] = "25000"
    env_with_args["LAST_PORT"] = "25100"

    output = subprocess.check_output(['bash', 'scripts/compose.sh', 'generate_json'], env=env_with_args)
    json_data = json.loads(output)
    register_command = utils.register_command(json_data, "some_key")

    checked_result = "./fluence register --node_ip 123 --tendermint_key 456 --tendermint_node_id qwe --contract_address ca --account oa --secret_key some_key --start_port 25000 --last_port 25100 --eth_url http://123:8545 --wait_syncing --base64_tendermint_key"

    assert register_command == checked_result
