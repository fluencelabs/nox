# Usage:
# make node -T TAG=v1.2.3

# tag defaults to latest
TAG ?= latest

node: ;DOCKER_BUILDKIT=1\
    docker build -t 'fluencelabs/node:$(TAG)' -f tools/docker/Node.Dockerfile .

node-test: ;DOCKER_BUILDKIT=1\
    docker build --build-arg environment=test -t 'fluencelabs/node:$(TAG)' -f tools/docker/Node.Dockerfile .

worker: ;DOCKER_BUILDKIT=1\
    docker build -t 'fluencelabs/statemachine:$(TAG)' -f tools/docker/Worker.Dockerfile .

dashboard: ;DOCKER_BUILDKIT=1\
    docker build -t 'fluencelabs/dashboard:$(TAG)' -f tools/docker/Dashboard.Dockerfile .

.PHONY: node node-test worker dashboard
