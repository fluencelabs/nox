# syntax=docker/dockerfile:1.1.3-experimental
# KEEP THE LINE ABOVE ^^^

# !!! REMEMBER to set `export DOCKER_BUILDKIT=1` before running docker build!
# !!! REMEMBER to use docker >= 19.03

ARG environment=production

############## Build for production
FROM mozilla/sbt as production
USER root
COPY . /fluence
WORKDIR /fluence
RUN apt-get update
RUN apt install -y build-essential gcc curl
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly-2019-09-23 && echo $PATH && sleep 11
RUN sbt statemachine-docker/assembly

############## Copy jar from local fs for tests, master-node.jar should be prebuilt
FROM scratch as test
COPY . /fluence

############## Aux dynamic stage, could be either test or production
FROM $environment as build

############## Build final image
FROM openjdk:8-jre-alpine
VOLUME /worker
COPY --from=build /fluence/statemachine/docker/worker /worker
COPY --from=build /fluence/statemachine/docker/target/scala-2.12/statemachine.jar /statemachine.jar
COPY --from=build /fluence/vm/frank/target/release/libfrank.so /native/x86_64-linux/libfrank.so

ENTRYPOINT ["sh", "/worker/run.sh", "/statemachine.jar"]
