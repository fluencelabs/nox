ARG IPFS_VERSION=0.23.0

# ipfs
FROM ipfs/go-ipfs:v${IPFS_VERSION} as prepare-ipfs

# base image
FROM ubuntu:jammy

ARG TARGETARCH

# https://github.com/opencontainers/image-spec/blob/main/annotations.md#pre-defined-annotation-keys
LABEL org.opencontainers.image.base.name="ubuntu:jammy"
LABEL org.opencontainers.image.url="https://github.com/fluencelabs/nox"
LABEL org.opencontainers.image.vendor="fluencelabs"
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
    ca-certificates \
  	jq \
  	less \
  	curl wget && \
  apt-get clean && \
  rm -rf \
  	/tmp/* \
  	/var/lib/apt/lists/* \
  	/var/tmp/*

# copy IPFS binary
COPY --from=prepare-ipfs /usr/local/bin/ipfs /usr/bin/ipfs

# copy nox binary
COPY ./binaries/nox-${TARGETARCH}/nox /usr/bin/nox
RUN chmod +x /usr/bin/nox
# copy default fluence config
COPY Config.default.toml /.fluence/v1/Config.toml
# copy entrypoint script
COPY entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
