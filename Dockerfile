from bitnami/minideb:latest

ARG exe=
ARG config=
ARG air_interpreter=

copy $exe /executable
run chmod +x /executable

copy $config /.fluence/Config.toml
copy $air_interpreter /.fluence/stepper/modules

volume /wasm_modules

env RUST_LOG="info,aquamarine=warn,tokio_threadpool=info,tokio_reactor=info,mio=info,tokio_io=info,soketto=info,yamux=info,multistream_select=info,libp2p_secio=info,libp2p_websocket::framed=info,libp2p_ping=info,libp2p_core::upgrade::apply=info,libp2p_kad::kbucket=info,cranelift_codegen=info,wasmer_wasi=info,cranelift_codegen=info,wasmer_wasi=info"
env RUST_BACKTRACE="1"

entrypoint ["/executable"]
