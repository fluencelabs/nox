#from rust as build
#copy . /project
#workdir /project
#run cargo build -p janus-server --release

from rust
#copy --from=build /project/target/release/janus-server /janus-server
copy ./target/x86_64-unknown-linux-gnu/release/janus-server /janus-server
entrypoint ["/janus-server"]
