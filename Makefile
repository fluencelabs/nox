release:
	cargo build --release

build: release

test:
	# run tests on release because current WASM runtime is too slow on debug
	cargo test --release

server:
    WASM_LOG="trace" \
	RUST_LOG="debug,\
	aquamarine::aqua_runtime=off,\
	ipfs_effector=off,\
	ipfs_pure=off,\
	system_services=debug,\
	marine_core::module::marine_module=info,\
	aquamarine=warn,\
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
	libp2p_kad::kbucket=info,\
	cranelift_codegen=info,\
	wasmer_wasi=info,\
	cranelift_codegen=info,\
	wasmer_wasi=info,\
	run-console=trace,\
	wasmtime_cranelift=off,\
	wasmtime_jit=off,\
	particle_reap=off" \
	cargo run --release -p nox

local-env:
	docker compose -f docker-compose.yml up -d

local-env-logs:
	docker compose -f docker-compose.yml logs -f

local-nox:
	FLUENCE_ENV_AQUA_IPFS_EXTERNAL_API_MULTIADDR="/ip4/127.0.0.1/tcp/5001" \
	FLUENCE_ENV_CONNECTOR_API_ENDPOINT=http://127.0.0.1:8545 \
	FLUENCE_ENV_CONNECTOR_CONTRACT_ADDRESS="0xea6777e8c011E7968605fd012A9Dd49401ec386C" \
	FLUENCE_ENV_CONNECTOR_FROM_BLOCK=earliest \
	FLUENCE_ENV_AQUA_IPFS_LOCAL_API_MULTIADDR="/ip4/127.0.0.1/tcp/5001" \
	FLUENCE_SYSTEM_SERVICES__AQUA_IPFS__IPFS_BINARY_PATH="/opt/homebrew/bin/ipfs" \
	FLUENCE_SYSTEM_SERVICES__ENABLE="aqua-ipfs" \
	WASM_LOG="trace" \
	RUST_LOG="debug,\
	aquamarine::aqua_runtime=error,\
	ipfs_effector=off,\
	ipfs_pure=off,\
	system_services=debug,\
	marine_core::module::marine_module=info,\
	aquamarine=warn,\
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
	libp2p_kad::kbucket=info,\
	cranelift_codegen=info,\
	wasmer_wasi=info,\
	cranelift_codegen=info,\
	wasmer_wasi=info,\
	run-console=trace,\
	wasmtime_cranelift=off,\
	wasmtime_jit=off,\
	libp2p_tcp=off,\
	libp2p_swarm=off,\
	particle_reap=off" \
	cargo run --release -p nox -- --secret-key "74c9Fl8I+XFwlTRnLAyYlSML+Jk6zIkZgtQoo5deuGk="

.PHONY: server server-debug test release build deploy local-nox local-env local-env-logs
