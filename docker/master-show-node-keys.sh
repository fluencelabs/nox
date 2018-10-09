#!/bin/bash
# param
# $1 keys_location

echo "{~node_id~:~$(tendermint show_node_id --home=$1)~,~validator~:$(tendermint show_validator --home=$1)}" | sed 's/~/"/g'
