# syntax=docker/dockerfile:1.1.2-experimental
# KEEP THE LINE ABOVE ^^^

# !!! REMEMBER to set `export DOCKER_BUILDKIT=1` before running docker build!
# !!! REMEMBER to use docker >= 19.03

FROM mozilla/sbt
COPY . /fluence
WORKDIR /fluence
RUN --mount=type=cache,target=$HOME/.ivy2 sbt node/assembly
