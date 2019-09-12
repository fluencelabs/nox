# Usage:
# make node TAG=v1.2.3
# TAG is optional

# tag defaults to latest
TAG ?= latest
# enable docker kit
KIT      = DOCKER_BUILDKIT=1
BUILD    = $(KIT) docker build
TEST_ENV = --quiet --build-arg environment=test

NODE_FILE = tools/docker/Node.Dockerfile
WORK_FILE = tools/docker/Worker.Dockerfile
DASH_FILE = tools/docker/Dashboard.Dockerfile

NODE_IMG = 'fluencelabs/node:$(TAG)'
WORK_IMG = 'fluencelabs/statemachine:$(TAG)'
DASH_IMG = 'fluencelabs/dashboard:$(TAG)'

node:        ;$(BUILD)             -t $(NODE_IMG) -f $(NODE_FILE) .
node-test:   ;$(BUILD) $(TEST_ENV) -t $(NODE_IMG) -f $(NODE_FILE) .
worker:      ;$(BUILD)             -t $(WORK_IMG) -f $(WORK_FILE) .
worker-test: ;$(BUILD) $(TEST_ENV) -t $(WORK_IMG) -f $(WORK_FILE) .
dashboard:   ;$(BUILD)             -t $(DASH_IMG) -f $(DASH_FILE) .

.PHONY: node node-test worker worker-test dashboard
