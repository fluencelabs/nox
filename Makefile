release:
	cargo +nightly build --release

build: release

test:
	# run tests on release because current WASM runtime is too slow on debug
	cargo +nightly test --release

server:
	RUST_LOG="info,tide=off" \
	cargo +nightly run -p particle-server -- -c ./deploy/Config.default.toml

server-debug:
	RUST_LOG="debug,\
	tide=off,\
	cranelift_codegen=info,\
    yamux::connection::stream=info,\
    tokio_threadpool=info,\
    tokio_reactor=info,\
    mio=info,\
    tokio_io=info,\
    soketto=info,\
    yamux=info,\
    multistream_select=info,\
    libp2p_secio=info,\
    libp2p_websocket::framed=info,\
    libp2p_ping=info,\
    libp2p_core::upgrade::apply=info,\
    libp2p_plaintext=info,\
    cranelift_codegen=info,\
    wasmer_wasi=info,\
    wasmer_interface_types_fl=info,\
    async_std=info,\
    async_io=info,\
    polling=info" \
	cargo +nightly run -p particle-server -- -c ./deploy/Config.default.toml

# build x86_64 binaries
cross-build:
	command -v cross || cargo install cross
	cargo +nightly update --aggressive
	cross +nightly build --release --target x86_64-unknown-linux-gnu

X86_TARGET=./target/x86_64-unknown-linux-gnu/release

SERVER_EXE = ${X86_TARGET}/particle-server
SERVER=--build-arg local_exe=${SERVER_EXE} --build-arg exe=particle-server
BRANCH := $(shell git rev-parse --abbrev-ref HEAD)

# bundle containers from existing binaries
containers:
	docker build ${SERVER} -t fluencelabs/fluence:${BRANCH} .

# build binaries, then bundle containers
docker: cross-build containers

# push containers to dockerhub
push:
	docker push fluencelabs/fluence:${BRANCH}

# build binaries, bundle containers, then push them
docker-push: docker push

# deploy existing containers (configure tag in deployment_config.json)
deploy:
	cd deploy; fab deploy_fluence

# rebuild binaries and containers, push and deploy
docker-deploy: docker-push deploy

# bundle containers from existing binaries, push and deploy
containers-deploy: containers push deploy

.PHONY: server server-debug docker docker-push clean test release build deploy docker-push-deploy
