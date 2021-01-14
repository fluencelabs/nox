release:
	cargo +nightly build --release

build: release

test:
	# run tests on release because current WASM runtime is too slow on debug
	cargo +nightly test --release

server:
	RUST_LOG="info,tide=off" \
	cargo +nightly run --release -p particle-server -- -c ./deploy/Config.default.toml

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
	cargo +nightly run --release -p particle-server -- -c ./deploy/Config.default.toml

# deploy existing containers (configure tag in deployment_config.json)
deploy:
	cd deploy; fab deploy_fluence

.PHONY: server server-debug test release build deploy
