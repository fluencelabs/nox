from bitnami/minideb:latest

ARG exe=
ARG config=

copy $exe /fluence
run chmod +x /fluence

copy $config /.fluence/Config.toml

volume /.fluence

env RUST_LOG="info,aquamarine=warn,tokio_threadpool=info,tokio_reactor=info,mio=info,tokio_io=info,soketto=info,yamux=info,multistream_select=info,libp2p_secio=info,libp2p_websocket::framed=info,libp2p_ping=info,libp2p_core::upgrade::apply=info,libp2p_kad::kbucket=info,cranelift_codegen=info,wasmer_wasi=info,cranelift_codegen=info,wasmer_wasi=info"
env RUST_BACKTRACE="1"

entrypoint ["/fluence"]
