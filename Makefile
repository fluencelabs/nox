rust: ;RUST_LOG="trace,tokio_threadpool=info,tokio_reactor=info,mio=info,tokio_io=info" cargo run

rust-dial: ;RUST_LOG="trace,tokio_threadpool=info,tokio_reactor=info,mio=info,tokio_io=info" cargo run dial '/ip4/127.0.0.1/tcp/30000'

js: npm-install; cd js ; DEBUG="*,-latency-monitor:LatencyMonitor,-libp2p:connection-manager" node run.js

js-rust-peerid: npm-install; cd js; DEBUG="*,-latency-monitor:LatencyMonitor,-libp2p:connection-manager" node run.js /ip4/127.0.0.1/tcp/30000/p2p/QmTESkr2vWDCKqiHVsyvf4iRQCBgvNDqBJ6P3yTTDb6haw

npm-install: ;cd js; npm install

.PHONY: rust js js-rust-peerid npm-install
