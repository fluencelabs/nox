ARG IPFS_VERSION=0.23.0

ARG TARGETPLATFORM
ARG BUILDPLATFORM

# ipfs
FROM --platform=$TARGETPLATFORM ipfs/go-ipfs:v${IPFS_VERSION} as prepare-ipfs

# build
FROM rust as build

RUN cargo build -p nox --release

# base image
FROM --platform=$TARGETPLATFORM ghcr.io/linuxserver/baseimage-ubuntu:jammy

# https://github.com/opencontainers/image-spec/blob/main/annotations.md#pre-defined-annotation-keys
LABEL org.opencontainers.image.base.name="ghcr.io/linuxserver/baseimage-ubuntu:jammy"
LABEL org.opencontainers.image.url="https://github.com/fluencelabs/nox"
LABEL org.opencontainers.image.vendor="fluencelabs"
LABEL maintainer="fluencelabs"
LABEL org.opencontainers.image.authors="fluencelabs"
LABEL org.opencontainers.image.title="Nox"
LABEL org.opencontainers.image.description="Rust implementation of the Fluence network peer"

ENV RUST_LOG="info"
ENV RUST_BACKTRACE="1"

# install necessary packages
RUN \
  apt-get update && \
  apt-get install -y --no-install-recommends \
    gosu \
  	jq \
  	less \
  	curl wget && \
  apt-get clean && \
  rm -rf \
  	/tmp/* \
  	/var/lib/apt/lists/* \
  	/var/tmp/*

# add fluence user
ENV FLUENCE_UID=911 FLUENCE_GID=1000
RUN adduser --uid ${FLUENCE_UID} --gid ${FLUENCE_GID} --no-create-home --disabled-password --gecos "" fluence

# copy IPFS binary
COPY --from=prepare-ipfs /usr/local/bin/ipfs /usr/bin/ipfs

# copy default fluence config
ENV FLUENCE_BASE_DIR=/.fluence
COPY fluence/Config.default.toml ${FLUENCE_BASE_DIR}/v1/Config.toml
ENV FLUENCE_CONFIG=${FLUENCE_BASE_DIR}/v1/Config.toml
# copy entrypoint script
COPY entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
