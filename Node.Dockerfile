# syntax=docker/dockerfile:1.1.2-experimental

FROM mozilla/sbt
COPY . /fluence
WORKDIR /fluence
RUN --mount=type=cache,target=$HOME/.ivy2 sbt node/assembly
