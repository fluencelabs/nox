# Usage:
# make node -T TAG=v1.2.3

# defaults to latest
TAG ?= latest

node: ;DOCKER_BUILDKIT=1\
    docker build -t 'fluencelabs/node:$(TAG)' -f node/Node.Dockerfile .

worker: ;DOCKER_BUILDKIT=1\
    docker build -t 'fluencelabs/statemachine:$(TAG)' -f statemachine/Worker.Dockerfile .

dashboard: ;DOCKER_BUILDKIT=1\
    docker build -t 'fluencelabs/dashboard:$(TAG)' -f dashboard/Dashboard.Dockerfile .

.PHONY: node worker dashboard
