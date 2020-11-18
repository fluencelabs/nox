from bitnami/minideb:latest

ARG exe=
ARG config=
ARG air_interpreter=

copy $exe /executable
run chmod +x /executable

copy $config /.fluence/Config.toml
copy $air_interpreter /.fluence/stepper/modules

volume /wasm_modules
entrypoint ["/executable"]
