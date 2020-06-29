include node_client.mk

BOOTSTRAP_NODE=/ip4/207.154.232.92/tcp/7777

default:
	cargo build

release:
	cargo build --release

build: release

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
IPFS_DOCKERFILE=./client/fluence-ipfs/docker/Dockerfile
IPFS_TAG=${BRANCH}-with-ipfs-multiaddr

docker: cross-build
	docker build ${SERVER} -t fluencelabs/fluence:${BRANCH} .
	docker build -t fluencelabs/fluence:${IPFS_TAG} -f ${IPFS_DOCKERFILE} .

push:
	docker push fluencelabs/fluence:${BRANCH}
	docker push fluencelabs/fluence:${IPFS_TAG}

docker-push: docker push

ENDURANCE_EXE=$(shell find ${X86_TARGET} -name "endurance*" -perm +111 -type f)
ENDURANCE=--build-arg exe=endurance --build-arg local_exe=${ENDURANCE_EXE}
endurance-docker:
	cargo update -p libp2p
	cross build --release --target x86_64-unknown-linux-gnu --test endurance
	docker build ${ENDURANCE} -t fluencelabs/fluence-endurance:${BRANCH} .
	docker push fluencelabs/fluence-endurance:${BRANCH}

deploy:
	cd deploy; fab deploy_fluence

docker-push-deploy: docker-push deploy

.PHONY: server server-debug docker docker-push clean test release build deploy
