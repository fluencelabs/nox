#!/bin/bash
# param
# $1 long_term_key_location

# create a JSON document with public keys
echo "{~node_id~:~$(tendermint show_node_id --home=$1)~,~validator~:$(tendermint show_validator --home=$1)}" | sed 's/~/"/g'
