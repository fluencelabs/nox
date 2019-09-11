# syntax=docker/dockerfile:1.1.3-experimental
# KEEP THE LINE ABOVE ^^^

# !!! REMEMBER to set `export DOCKER_BUILDKIT=1` before running docker build!
# !!! REMEMBER to use docker >= 19.03

FROM mozilla/sbt as build
USER root
COPY . /fluence
WORKDIR /fluence
RUN --mount=type=cache,target=/root/.ivy2 --mount=type=cache,target=/root/.sbt sbt node/assembly
ARG DOCKER_BINARY=https://download.docker.com/linux/static/stable/x86_64/docker-19.03.2.tgz
RUN wget -q $DOCKER_BINARY -O- | tar --strip-components=1 -zx docker/docker

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
#
COPY --from=build /fluence/node/src/main/resources/docker /
COPY --from=build /fluence/node/target/scala-2.12/master-node.jar /master-node.jar

CMD ["java", "-jar", "-Dconfig.file=/master/application.conf", "/master-node.jar"]
ENTRYPOINT ["sh", "/entrypoint.sh"]
