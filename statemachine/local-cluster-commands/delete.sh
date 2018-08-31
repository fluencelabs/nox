kill -9 $(ps aux | grep 'tendermint node'  | awk '{print $2}')
kill -9 $(ps aux | grep 'statemachine/run' | awk '{print $2}')

rm -rf ~/.tendermint1
rm -rf ~/.tendermint2
rm -rf ~/.tendermint3
rm -rf ~/.tendermint4
