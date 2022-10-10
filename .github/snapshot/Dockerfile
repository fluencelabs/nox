FROM ubuntu:22.04
COPY .github/snapshot/Config.toml .fluence/v1/Config.toml
COPY target/release/particle-node /fluence
ENV RUST_LOG="info,aquamarine=warn,tokio_threadpool=info,tokio_reactor=info,mio=info,tokio_io=info,soketto=info,yamux=info,multistream_select=info,libp2p_secio=info,libp2p_websocket::framed=info,libp2p_ping=info,libp2p_core::upgrade::apply=info,libp2p_kad::kbucket=info,cranelift_codegen=info,wasmer_wasi=info,cranelift_codegen=info,wasmer_wasi=info"
ENV RUST_BACKTRACE="1"
ADD https://github.com/fluencelabs/trust-graph/releases/download/v3.0.4/trust-graph.tar.gz trust-graph.tar.gz
RUN mkdir -p .fluence/v1/builtins && tar -C .fluence/v1/builtins -xf trust-graph.tar.gz && rm trust-graph.tar.gz
ENTRYPOINT ["/fluence"]
CMD ["--local"]
