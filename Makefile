release:
	cargo +nightly build --release

build: release

test:
	# run tests on release because current WASM runtime is too slow on debug
	cargo +nightly test --release

server:
	RUST_LOG="info,tide=off" \
	cargo +nightly run --release -p particle-node

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
	cargo +nightly run --release -p particle-node -- -c ./deploy/Config.default.toml

.PHONY: server server-debug test release build deploy
