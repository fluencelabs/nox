from rust:slim as build
copy . /project
workdir /project
run cargo build --release

from scratch
copy --from=build /project/target/release/fluence /fluence
cmd ["/fluence"]