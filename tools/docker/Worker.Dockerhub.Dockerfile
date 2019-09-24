### This docker file is a copy of Worker.Dockerfile, intended to be
### built in Dockerhub which doesn't support experimental syntax yet

ARG environment=production

############## Build for production
FROM mozilla/sbt as production
USER root
COPY . /fluence
WORKDIR /fluence
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
COPY --from=build /fluence/vm/frank/target/release/* /usr/lib/

ENTRYPOINT ["sh", "/worker/run.sh", "/statemachine.jar"]
