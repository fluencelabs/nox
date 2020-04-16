from rust
copy ./target/x86_64-unknown-linux-gnu/release/janus-server /janus-server
copy ./target/x86_64-unknown-linux-gnu/release/janus-ipfs /janus-ipfs
copy ./client/janus-ipfs/docker/run_janus_ipfs.sh /run_janus_ipfs.sh
copy ./client/janus-ipfs/docker/wait.sh /wait.sh
run chmod +x /run_janus_ipfs.sh
entrypoint ["/run_janus_ipfs.sh"]
