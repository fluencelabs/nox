BOOTSTRAP_NODE=/ip4/207.154.232.92/tcp/7777

default: ;cargo build

server: ;cargo run -p janus-server -- -b ${BOOTSTRAP_NODE}

server-debug: ;RUST_LOG="trace,tokio_threadpool=info,tokio_reactor=info,mio=info,tokio_io=info" cargo run -p janus-server -- -b ${BOOTSTRAP_NODE}

client: ;cargo run -p janus-client -- ${args}

client-debug: ;RUST_LOG="trace,tokio_threadpool=info,tokio_reactor=info,mio=info,tokio_io=info" cargo run -p janus-client -- ${args}

clean: ;cargo clean

.PHONY: server server-debug client client-debug
