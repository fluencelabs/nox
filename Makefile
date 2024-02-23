release:
	cargo build --release

build: release

test:
	# run tests on release because current WASM runtime is too slow on debug
	cargo test --release

server:
	WASM_LOG="trace" \
	RUST_LOG="info,\
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
	run-console=debug,\
	wasmtime_cranelift=off,\
	wasmtime_jit=off,\
	particle_reap=debug" \
	cargo run --release -p nox

spell:
	WASM_LOG="trace" \
	RUST_LOG="info,\
	aquamarine::aqua_runtime=off,\
	ipfs_effector=debug,\
	ipfs_pure=debug,\
	run-console=debug,\
	system_services=debug,\
	spell_even_bus=trace,\
	marine_core::module::marine_module=info,\
	aquamarine::log=debug,\
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
	wasmtime_cranelift=off,\
	wasmtime_jit=off,\
	particle_protocol=off,\
	particle_reap=debug" \
	cargo run --release -p nox

local-env:
	docker compose -f docker-compose.yml up -d

local-env-logs:
	docker compose -f docker-compose.yml logs -f

# mumbai 0x93A2897deDcC5478a9581808F5EC25F4FadbC312
# local 0x0f68c702dC151D07038fA40ab3Ed1f9b8BAC2981

local-nox:
	FLUENCE_ENV_AQUA_IPFS_EXTERNAL_API_MULTIADDR="/ip4/127.0.0.1/tcp/5001" \
    FLUENCE_ENV_CONNECTOR_API_ENDPOINT="http://127.0.0.1:8545" \
	FLUENCE_ENV_CONNECTOR_CONTRACT_ADDRESS="0x0f68c702dC151D07038fA40ab3Ed1f9b8BAC2981" \
	FLUENCE_ENV_CONNECTOR_WALLET_KEY="0xfdc4ba94809c7930fe4676b7d845cbf8fa5c1beae8744d959530e5073004cf3f" \
	FLUENCE_ENV_CONNECTOR_FROM_BLOCK=earliest \
	FLUENCE_ENV_AQUA_IPFS_LOCAL_API_MULTIADDR="/ip4/127.0.0.1/tcp/5001" \
	FLUENCE_SYSTEM_SERVICES__DECIDER__DECIDER_PERIOD_SEC=60 \
	FLUENCE_MAX_SPELL_PARTICLE_TTL="55s" \
	FLUENCE_SYSTEM_SERVICES__DECIDER__NETWORK_ID=31337 \
	FLUENCE_SYSTEM_SERVICES__AQUA_IPFS__IPFS_BINARY_PATH="$(shell which ipfs)" \
	FLUENCE_SYSTEM_SERVICES__ENABLE="aqua-ipfs,decider" \
	WASM_LOG="trace" \
	RUST_LOG="debug,\
		execution=trace,\
		expired=info,\
		effectors=info,\
		dispatcher=info,\
		aquamarine::plumber=info,\
		aquamarine::partice_functions=debug,\
		aquamarine::log=debug,\
		aquamarine=info,\
		aquamarine::aqua_runtime=warn,\
		aquamarine::particle_executor=warn,\
		ipfs_effector=off,\
		ipfs_pure=off,\
		system_services=debug,\
		marine_core::module::marine_module=info,\
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
		particle_protocol::libp2p_protocol::upgrade=info,\
		libp2p_mplex=off,\
		particle_reap=debug" \
	cargo run --release -p nox -- -k "2WijTVdhVRzyZamWjqPx4V4iNMrajegNMwNa2PmvPSZV6RRpo5M2fsPWdQr22HVRubuJhhSw8BrWiGt6FPhFAuXy" --aqua-pool-size 3 --ws-port 9992

.PHONY: server server-debug test release build deploy local-nox local-env local-env-logs
