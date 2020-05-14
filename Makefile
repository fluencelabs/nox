include node_client.mk

BOOTSTRAP_NODE=/ip4/207.154.232.92/tcp/7777

default:
	cargo build

release:
	cargo build --release

test:
	cargo test

server:
	cargo run -p fluence-server -- -b ${BOOTSTRAP_NODE}

server-debug:
	RUST_LOG="trace,tokio_threadpool=info,tokio_reactor=info,mio=info,tokio_io=info" \
	cargo run -p fluence-server -- -b ${BOOTSTRAP_NODE}

clean:
	cargo clean

cross-build:
	cargo update -p libp2p
	cross build --release --target x86_64-unknown-linux-gnu

X86_TARGET=./target/x86_64-unknown-linux-gnu/release

SERVER_EXE = ${X86_TARGET}/fluence-server
SERVER=--build-arg local_exe=${SERVER_EXE} --build-arg exe=fluence-server
BRANCH := $(shell git rev-parse --abbrev-ref HEAD)
JI_DOCKERFILE=./client/fluence-ipfs/docker/Dockerfile
JI_TAG=${BRANCH}-with-ipfs-multiaddr
docker: cross-build
	docker build ${SERVER} -t folexflu/fluence:${BRANCH} .
	docker push folexflu/fluence:${BRANCH}
	docker build -t folexflu/fluence:${JI_TAG} -f ${JI_DOCKERFILE} .
	docker push folexflu/fluence:${JI_TAG}

ENDURANCE_EXE=$(shell find ${X86_TARGET} -name "endurance*" -perm +111 -type f)
ENDURANCE=--build-arg exe=endurance --build-arg local_exe=${ENDURANCE_EXE}
endurance-docker:
	cargo update -p libp2p
	cross build --release --target x86_64-unknown-linux-gnu --test endurance
	docker build ${ENDURANCE} -t folexflu/fluence-endurance:${BRANCH} .
	docker push folexflu/fluence-endurance:${BRANCH}

.PHONY: server server-debug docker clean test release
