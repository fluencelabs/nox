from minideb:latest

ARG exe=particle-server
ARG local_exe=
ARG config=

copy $local_exe /executable
run chmod +x /executable

copy $config /.fluence/Config.toml

volume /wasm_modules
entrypoint ["/executable"]
