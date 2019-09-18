### This docker file is a copy of Node.Dockerfile, intended to be
### built in Dockerhub which doesn't support experimental syntax yet

ARG environment=production

############## Build for production
FROM mozilla/sbt as production
USER root
COPY . /fluence
WORKDIR /fluence
RUN sbt node/assembly

############## Download docker binary
FROM alpine as docker
ARG DOCKER_BINARY=https://download.docker.com/linux/static/stable/x86_64/docker-19.03.2.tgz
RUN wget $DOCKER_BINARY -O- | tar -C / --strip-components=1 -zx docker/docker

############## Copy jar from local fs for tests, master-node.jar should be prebuilt
FROM scratch as test
COPY . /fluence

############## Aux dynamic stage, could be either test or production
FROM $environment as build

############## Build final image
FROM openjdk:8-jre-alpine

# this is needed for some binaries (e.g. rocksdb) to run properly on alpine linux since they need libc and alpine uses musl
RUN ln -sf /lib/libc.musl-x86_64.so.1 /usr/lib/ld-linux-x86-64.so.2

VOLUME /master
ENV MIN_PORT 10000
ENV MAX_PORT 11000
EXPOSE 5678

# The following directory structure is assumed in node/src/main/resources:
#    docker/
#      entrypoint.sh
#      application.conf

COPY --from=docker /docker /usr/bin/docker
COPY --from=build  /fluence/node/src/main/resources/docker /
COPY --from=build  /fluence/node/target/scala-2.12/master-node.jar /master-node.jar

CMD ["java", "-jar", "-Dconfig.file=/master/application.conf", "/master-node.jar"]
ENTRYPOINT ["sh", "/entrypoint.sh"]
