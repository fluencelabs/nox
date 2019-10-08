# Fluence docker files
This directory contains docker files to build:
- Fluence Node
- Fluence Statemachine (aka Worker)
- Fluence dashboard
- SBT + Rust image

# Dockerhub
There are some dockerfiles in `dockerhub` directore named as `*.Dockerhub.Dockerfile` – these are intended to be built on Dockerhub, and are just copies of corresponding docker files, but with experimental features removed

# How to build
Preferred way is to use `Makefile` in the root of the project to build docker images. If you want to use `docker build` directly, please note that it is recommended to enable Buildkit via `DOCKER_BUILDKIT=1` environment variable.
