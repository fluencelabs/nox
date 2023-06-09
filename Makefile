release:
	cargo build --release

build: release

test:
	# run tests on release because current WASM runtime is too slow on debug
	cargo test --release

server:
	RUST_LOG="info,tide=off,tracing=off,avm_server=off,run-console=debug,system_services=debug,sorcerer::spell_builtins=debug,sorcerer=debug" \
	cargo run --release -p nox

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
    polling=info, \
    avm_server=off,\
    tracing=off"\
	cargo run --release -p nox -- -c ./deploy/Config.default.toml

.PHONY: server server-debug test release build deploy
