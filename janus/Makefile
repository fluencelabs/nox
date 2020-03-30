BOOTSTRAP_NODE=/ip4/207.154.232.92/tcp/7777

default: ;cargo build

release: ;cargo build --release

test: ;cargo test

server: ;cargo run -p janus-server -- -b ${BOOTSTRAP_NODE}

server-debug: ;RUST_LOG="trace,tokio_threadpool=info,tokio_reactor=info,mio=info,tokio_io=info" cargo run -p janus-server -- -b ${BOOTSTRAP_NODE}

client: ;cargo run -p janus-client -- ${args}

client-debug: ;RUST_LOG="trace,tokio_threadpool=info,tokio_reactor=info,mio=info,tokio_io=info" cargo run -p janus-client -- ${args}

clean: ;cargo clean

BRANCH := $(shell git rev-parse --abbrev-ref HEAD)
docker: ;
	cargo update -p libp2p
	cross build --release --target x86_64-unknown-linux-gnu
	docker build -t folexflu/janus:${BRANCH} .
	docker push folexflu/janus:${BRANCH}

node-bootstrap:
	RUST_BACKTRACE=1 RUST_LOG="janus_server=trace" \
	./target/debug/janus-server -n 7770 -o 9990 \
	-k 5uzSH9jWFWJAaG6XdeVKR1ubgd2ca5HjaRd29fXbX3kMyJP8KPkRP3TKwV3S679urWytvpkSS8LXRaTavjF4GPQY

node-first:
	RUST_BACKTRACE=1 RUST_LOG="janus_server=trace" \
	./target/debug/janus-server -n 7771 -o 9991 -b /ip4/127.0.0.1/tcp/7770 \
	-k 55tL1A8oVXn8VwWG4VXCiurRiBbRokzWnQZSauKnf6TCCtY7bp9m7HUE4JLEB71Sp7oA42mJGhN65AaZ9pCg2FQE

node-second:
	RUST_BACKTRACE=1 RUST_LOG="janus_server=trace" \
	./target/debug/janus-server -n 7772 -o 9992 -b /ip4/127.0.0.1/tcp/7770 \
	-k 2Gs9GH37cwzJoMXgaZa99wjwBHA7HE8fZRxTquyXsCVwQQ7NCyAZRdBfjEc3GHcFoN2VVJiseB4wapqd5ir9H8jB

client-first:
	./target/debug/janus-client \
		/ip4/127.0.0.1/tcp/9991/ws \
		QmSY8FNMuXJNHwAneTHxDks51DcyqQ8TRnpe4b8SF1ZQnh

client-second:
	./target/debug/janus-client \
		/ip4/127.0.0.1/tcp/9992/ws \
		QmUQTSjNJig8ZEYiodbZEFzTC9WJa9aiX4G9nMaieXhgUF

node-tmux:
	cargo update -p libp2p
	cargo build
	tmux \
        new-session  'make node-bootstrap' \; \
        split-window 'sleep 1 && make node-first' \; \
        split-window 'sleep 2 && make node-second' \; \
        setw synchronize-panes \; \
        select-layout tiled

client-tmux:
	tmux \
		new-session  'make client-first' \; \
		split-window 'sleep 1 && make client-second' \; \
		select-layout tiled

.PHONY: server server-debug client client-debug
