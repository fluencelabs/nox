# Usage:
# make node -T TAG=v1.2.3

# tag defaults to latest
TAG ?= latest
# enable docker kit
KIT = DOCKER_BUILDKIT=1
TEST_ENV = --build-arg environment=test

NODE_FILE = tools/docker/Node.Dockerfile
WORK_FILE = tools/docker/Worker.Dockerfile
DASH_FILE = tools/docker/Dashboard.Dockerfile

NODE_IMG = 'fluencelabs/node:$(TAG)'
WORK_IMG = 'fluencelabs/statemachine:$(TAG)'
DASH_IMG = 'fluencelabs/dashboard:$(TAG)'

node:        ;$(KIT) docker build             -t $(NODE_IMG) -f $(NODE_FILE) .
node-test:   ;$(KIT) docker build $(TEST_ENV) -t $(NODE_IMG) -f $(NODE_FILE) .
worker:      ;$(KIT) docker build             -t $(WORK_IMG) -f $(WORK_FILE) .
worker-test: ;$(KIT) docker build $(TEST_ENV) -t $(WORK_IMG) -f $(WORK_FILE) .
dashboard:   ;$(KIT) docker build             -t $(DASH_IMG) -f $(DASH_FILE) .

.PHONY: node node-test worker dashboard
