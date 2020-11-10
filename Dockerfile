from rust
ARG exe=particle-server
ARG local_exe=
copy $local_exe /executable
run chmod +x /executable
volume /wasm_modules
entrypoint ["/executable"]
