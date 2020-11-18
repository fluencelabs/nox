from bitnami/minideb:latest

ARG exe=
ARG config=

copy $exe /executable
run chmod +x /executable

copy $config /.fluence/Config.toml

volume /wasm_modules
entrypoint ["/executable"]
