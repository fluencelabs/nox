kill $(ps aux | grep 'tendermint node' | awk '{print $2}')
kill $(ps aux | grep 'run 46.58' | awk '{print $2}')

rm -rf ~/.tendermint1
rm -rf ~/.tendermint2
rm -rf ~/.tendermint3
rm -rf ~/.tendermint4
