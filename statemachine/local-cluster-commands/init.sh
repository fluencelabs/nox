tendermint init --home $HOME/.tendermint1
tendermint init --home $HOME/.tendermint2
tendermint init --home $HOME/.tendermint3
tendermint init --home $HOME/.tendermint4
# tendermint show_node_id --home $HOME/.tendermint1
# tendermint show_node_id --home $HOME/.tendermint2
# tendermint show_node_id --home $HOME/.tendermint3
# tendermint show_node_id --home $HOME/.tendermint4
# tendermint show_validator --home $HOME/.tendermint1
# tendermint show_validator --home $HOME/.tendermint2
# tendermint show_validator --home $HOME/.tendermint3
# tendermint show_validator --home $HOME/.tendermint4

TM_VALIDATOR1='{"pub_key":'$(tendermint show_validator --home $HOME/.tendermint1)',"power":"10","name":""}'
TM_VALIDATOR2='{"pub_key":'$(tendermint show_validator --home $HOME/.tendermint2)',"power":"10","name":""}'
TM_VALIDATOR3='{"pub_key":'$(tendermint show_validator --home $HOME/.tendermint3)',"power":"10","name":""}'
TM_VALIDATOR4='{"pub_key":'$(tendermint show_validator --home $HOME/.tendermint4)',"power":"10","name":""}'
TM_VALIDATORS=$TM_VALIDATOR1,$TM_VALIDATOR2,$TM_VALIDATOR3,$TM_VALIDATOR4

tr -d ' \n' < $HOME/.tendermint1/config/genesis.json > $HOME/.tendermint1/config/genesis.json.tmp
mv $HOME/.tendermint1/config/genesis.json.tmp $HOME/.tendermint1/config/genesis.json
sed -i -e 's#'$TM_VALIDATOR1'#'$TM_VALIDATORS'#g' $HOME/.tendermint1/config/genesis.json

cp $HOME/.tendermint1/config/genesis.json $HOME/.tendermint2/config/genesis.json
cp $HOME/.tendermint1/config/genesis.json $HOME/.tendermint3/config/genesis.json
cp $HOME/.tendermint1/config/genesis.json $HOME/.tendermint4/config/genesis.json
# cat $HOME/.tendermint4/config/genesis.json

sed -i -e 's#addr_book_strict = true#addr_book_strict = false#g' $HOME/.tendermint1/config/config.toml
sed -i -e 's#addr_book_strict = true#addr_book_strict = false#g' $HOME/.tendermint2/config/config.toml
sed -i -e 's#addr_book_strict = true#addr_book_strict = false#g' $HOME/.tendermint3/config/config.toml
sed -i -e 's#addr_book_strict = true#addr_book_strict = false#g' $HOME/.tendermint4/config/config.toml

sed -i -e 's#cache_size = 100000#cache_size = 0#g' $HOME/.tendermint1/config/config.toml
sed -i -e 's#cache_size = 100000#cache_size = 0#g' $HOME/.tendermint2/config/config.toml
sed -i -e 's#cache_size = 100000#cache_size = 0#g' $HOME/.tendermint3/config/config.toml
sed -i -e 's#cache_size = 100000#cache_size = 0#g' $HOME/.tendermint4/config/config.toml

export TM_PERSISTENT_PEERS=$(tendermint show_node_id --home $HOME/.tendermint1)"@0.0.0.0:46156,"$(tendermint show_node_id --home $HOME/.tendermint2)"@0.0.0.0:46256,"$(tendermint show_node_id --home $HOME/.tendermint3)"@0.0.0.0:46356,"$(tendermint show_node_id --home $HOME/.tendermint4)"@0.0.0.0:46456"
