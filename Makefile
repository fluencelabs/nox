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

# mumbai 0x93A2897deDcC5478a9581808F5EC25F4FadbC312
# local 0x0f68c702dC151D07038fA40ab3Ed1f9b8BAC2981

local-nox:
	FLUENCE_ENV_AQUA_IPFS_EXTERNAL_API_MULTIADDR="/ip4/127.0.0.1/tcp/5001" \
	FLUENCE_ENV_CONNECTOR_API_ENDPOINT="https://endpoints.omniatech.io/v1/matic/mumbai/public" \
	FLUENCE_ENV_CONNECTOR_CONTRACT_ADDRESS="0x48772E71Ee51946beC258D4127eDDF110A7dCbeD" \
	FLUENCE_ENV_CONNECTOR_WALLET_KEY="8447c3b0ebbb09f3153ec59702459ee23c4af524e7882de9f5718f14c3f0e00b" \
	FLUENCE_ENV_CONNECTOR_FROM_BLOCK=latest \
	FLUENCE_ENV_AQUA_IPFS_LOCAL_API_MULTIADDR="/ip4/127.0.0.1/tcp/5001" \
	FLUENCE_SYSTEM_SERVICES__DECIDER__DECIDER_PERIOD_SEC=30 \
	FLUENCE_SYSTEM_SERVICES__AQUA_IPFS__IPFS_BINARY_PATH="$(shell which ipfs)" \
	FLUENCE_SYSTEM_SERVICES__ENABLE="aqua-ipfs,decider" \
	WASM_LOG="trace" \
	RUST_LOG="debug,\
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
		particle_reap=off" \
	cargo run -p nox -- --secret-key "74c9Fl8I+XFwlTRnLAyYlSML+Jk6zIkZgtQoo5deuGk=" --aqua-pool-size 3

.PHONY: server server-debug test release build deploy local-nox local-env local-env-logs
