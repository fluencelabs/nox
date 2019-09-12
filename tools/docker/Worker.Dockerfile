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
RUN --mount=type=cache,target=/root/.ivy2 --mount=type=cache,target=/root/.sbt sbt statemachine-docker/assembly

############## Copy jar from local fs for tests, master-node.jar should be prebuilt
FROM scratch as test
COPY . /fluence

############## Aux dynamic stage, could be either test or production
FROM $environment as build

############## Build final image
FROM openjdk:8-jre-alpine
VOLUME /worker
EXPOSE 26658
COPY --from=build /fluence/statemachine/docker/worker /worker
COPY --from=build /fluence/statemachine/docker/target/scala-2.12/statemachine.jar /statemachine.jar
ENTRYPOINT ["sh", "/worker/run.sh", "/statemachine.jar"]
