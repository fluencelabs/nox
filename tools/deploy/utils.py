# creates register command to register deployed node
def register_command(data, secret_key):
    eth_url = "http://" + data['node_ip'] + ":8545"
    command = "./fluence register \
        --node_ip            %s \
        --tendermint_key     %s \
        --tendermint_node_id %s \
        --contract_address   %s \
        --account            %s \
        --secret_key         %s \
        --start_port         %s \
        --last_port          %s \
        --eth_url            %s \
        --wait_syncing \
        --gas_price 10 \
        --base64_tendermint_key" % (data['node_ip'], data['tendermint_key'], data['tendermint_node_id'], data['contract_address'],
                                    data['account'], secret_key, data['start_port'], data['last_port'], eth_url)

    return ' '.join(command.split())
