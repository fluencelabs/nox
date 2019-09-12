# Usage:
# make node TAG=v1.2.3
# TAG is optional

# tag defaults to latest
TAG ?= latest
# enable docker kit
KIT      = DOCKER_BUILDKIT=1
BUILD    = $(KIT) docker build
QUIET    = --quiet
TEST_ENV = environment=test
TEST     = $(QUIET) --build-arg $(TEST_ENV)
TRAVIS  ?= false

DIR       = tools/docker
NODE_FILE = Node.Dockerfile
WORK_FILE = Worker.Dockerfile
DASH_FILE = Dashboard.Dockerfile

NODE_IMG = 'fluencelabs/node:$(TAG)'
WORK_IMG = 'fluencelabs/statemachine:$(TAG)'
DASH_IMG = 'fluencelabs/dashboard:$(TAG)'

node:        ;$(BUILD)         -t $(NODE_IMG) -f $(DIR)/$(NODE_FILE) .
worker:      ;$(BUILD)         -t $(WORK_IMG) -f $(DIR)/$(WORK_FILE) .
dashboard:   ;$(BUILD)         -t $(DASH_IMG) -f $(DIR)/$(DASH_FILE) .
node-test:   ;$(BUILD) $(TEST) -t $(NODE_IMG) -f $(DIR)/$(NODE_FILE) .
worker-test: ;$(BUILD) $(TEST) -t $(WORK_IMG) -f $(DIR)/$(WORK_FILE) .

# Using buildctl here because TravisCI doesn't support BuildKit
# (see https://travis-ci.community/t/docker-builds-are-broken-if-buildkit-is-used-docker-buildkit-1/2994)
ifeq ($(TRAVIS), true)
node-test:   NODE-bctl-test
worker-test: WORK-bctl-test
endif

# $* becomes what matched by % in `%-bctl-test`: NODE or WORK
%-bctl-test: ;buildctl build \
             --frontend dockerfile.v0 \
             --local context=. \
             --local dockerfile=$(DIR) \
             --opt filename=$($*_FILE) \
             --opt build-arg:$(TEST_ENV) \
             --output type=docker,name=$($*_IMG) | docker load


.PHONY: node node-test worker worker-test dashboard %-bctl-test
