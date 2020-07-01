from ipfs/go-ipfs:v0.6.0 as ipfs

from rust
copy --from=ipfs /usr/local/bin/ipfs /usr/local/bin/ipfs
copy ./target/x86_64-unknown-linux-gnu/release/fluence-server /fluence-server
copy ./deploy/fluence-ipfs/run_fluence_ipfs.sh /run_fluence_ipfs.sh
copy ./deploy/fluence-ipfs/wait.sh /wait.sh
run chmod +x /run_fluence_ipfs.sh
entrypoint ["/run_fluence_ipfs.sh"]
