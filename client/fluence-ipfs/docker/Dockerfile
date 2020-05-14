from rust
copy ./target/x86_64-unknown-linux-gnu/release/fluence-server /fluence-server
copy ./target/x86_64-unknown-linux-gnu/release/fluence-ipfs /fluence-ipfs
copy ./client/fluence-ipfs/docker/run_fluence_ipfs.sh /run_fluence_ipfs.sh
copy ./client/fluence-ipfs/docker/wait.sh /wait.sh
run chmod +x /run_fluence_ipfs.sh
entrypoint ["/run_fluence_ipfs.sh"]
