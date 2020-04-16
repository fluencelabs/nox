include node_client.mk

BOOTSTRAP_NODE=/ip4/207.154.232.92/tcp/7777

default:
	cargo build

release:
	cargo build --release

test:
	cargo test

server:
	cargo run -p janus-server -- -b ${BOOTSTRAP_NODE}

server-debug:
	RUST_LOG="trace,tokio_threadpool=info,tokio_reactor=info,mio=info,tokio_io=info" \
	cargo run -p janus-server -- -b ${BOOTSTRAP_NODE}

clean:
	cargo clean

cross-build:
	cargo update -p libp2p
	cross build --release --target x86_64-unknown-linux-gnu

BRANCH := $(shell git rev-parse --abbrev-ref HEAD)
JI_DOCKERFILE=./client/janus-ipfs/docker/Dockerfile
JI_TAG=${BRANCH}-with-ipfs-multiaddr
docker: cross-build
	docker build -t folexflu/janus:${BRANCH} .
	docker push folexflu/janus:${BRANCH}
	docker build -t folexflu/janus:${JI_TAG} -f ${JI_DOCKERFILE} .
	docker push folexflu/janus:${JI_TAG}

.PHONY: server server-debug docker clean test release
