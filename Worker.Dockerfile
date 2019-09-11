# syntax=docker/dockerfile:1.1.3-experimental
# KEEP THE LINE ABOVE ^^^

# !!! REMEMBER to set `export DOCKER_BUILDKIT=1` before running docker build!
# !!! REMEMBER to use docker >= 19.03

FROM mozilla/sbt as build
USER root
COPY . /fluence
WORKDIR /fluence
RUN --mount=type=cache,target=/root/.ivy2 --mount=type=cache,target=/root/.sbt sbt statemachine/assembly

FROM openjdk:8-jre-alpine
VOLUME /worker
EXPOSE 26658
COPY --from=build /fluence/statemachine/docker/worker /worker
COPY --from=build /fluence/statemachine/docker/target/scala-2.12/statemachine.jar /statemachine.jar
ENTRYPOINT ["sh", "/worker/run.sh", "/statemachine.jar"]
