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
	cargo run -p particle-server -- -b ${BOOTSTRAP_NODE}

server-debug:
	RUST_LOG="trace,tokio_threadpool=info,tokio_reactor=info,mio=info,tokio_io=info" \
	cargo run -p particle-server -- -b ${BOOTSTRAP_NODE}

clean:
	cargo clean

# build x86_64 binaries
cross-build:
	cargo update -p libp2p
	cross build --release --target x86_64-unknown-linux-gnu

X86_TARGET=./target/x86_64-unknown-linux-gnu/release

SERVER_EXE = ${X86_TARGET}/particle-server
SERVER=--build-arg local_exe=${SERVER_EXE} --build-arg exe=particle-server
BRANCH := $(shell git rev-parse --abbrev-ref HEAD)
IPFS_DOCKERFILE=./deploy/fluence-ipfs/Dockerfile
IPFS_TAG=${BRANCH}-with-ipfs

# bundle containers from existing binaries
containers:
	docker build ${SERVER} -t fluencelabs/fluence:${BRANCH} .
	docker build -t fluencelabs/fluence:${IPFS_TAG} -f ${IPFS_DOCKERFILE} .

# build binaries, then bundle containers
docker: cross-build containers

# push containers to dockerhub
push:
	docker push fluencelabs/fluence:${BRANCH}
	docker push fluencelabs/fluence:${IPFS_TAG}

# build binaries, bundle containers, then push them
docker-push: docker push

# deploy existing containers (configure tag in deployment_config.json)
deploy:
	cd deploy; fab deploy_fluence

# rebuild binaries and containers, push and deploy
docker-deploy: docker-push deploy

# bundle containers from existing binaries, push and deploy
containers-deploy: containers push deploy

ENDURANCE_EXE=$(shell find ${X86_TARGET} -name "endurance*" -perm +111 -type f)
ENDURANCE=--build-arg exe=endurance --build-arg local_exe=${ENDURANCE_EXE}
endurance-docker:
	cargo update -p libp2p
	cross build --release --target x86_64-unknown-linux-gnu --test endurance
	docker build ${ENDURANCE} -t fluencelabs/fluence-endurance:${BRANCH} .
	docker push fluencelabs/fluence-endurance:${BRANCH}

.PHONY: server server-debug docker docker-push clean test release build deploy docker-push-deploy
