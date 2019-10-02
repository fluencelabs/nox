# Usage:
# make node TAG=v1.2.3
# TAG is optional

# tag defaults to latest
TAG ?= latest
ENV ?= stage
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
WORK_IMG = 'fluencelabs/worker:$(TAG)'
DASH_IMG = 'fluencelabs/dashboard:$(TAG)'

######### Build tasks #########
node:        ;$(BUILD)         -t $(NODE_IMG) -f $(DIR)/$(NODE_FILE) .
worker:      ;$(BUILD)         -t $(WORK_IMG) -f $(DIR)/$(WORK_FILE) .
dashboard:   ;$(BUILD)         -t $(DASH_IMG) -f $(DIR)/$(DASH_FILE) .
ifeq ($(TRAVIS), false)
node-test:   ;$(BUILD) $(TEST) -t $(NODE_IMG) -f $(DIR)/$(NODE_FILE) .
worker-test: ;$(BUILD) $(TEST) -t $(WORK_IMG) -f $(DIR)/$(WORK_FILE) .
endif

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

######### Deployment tasks #########

# Deploy from existing assembly jars
deploy-prebuilt: push-local deploy
# Rebuild jars and deploy
deploy-rebuild:  jars push-local deploy
# Build containers in docker build environment
deploy-clean:    push-clean deploy
deploy:          ;cd tools/deploy; fab --set environment=$(ENV),image_tag=$(TAG) deploy

push:            push-node push-worker
# Build containers from existing jars and publish
push-local:      node-test worker-test; $(MAKE) push
# Build containers in docker build environment and publish
push-clean:      node worker; $(MAKE) push
push-node:       ;docker push $(NODE_IMG)
push-worker:     ;docker push $(WORK_IMG)

# Build jars
jars:            ;sbt node/assembly statemachine-docker/assembly

######### Service containers #########
TOOLCHAIN ?= nightly-2019-09-23
RS_IMG     = fluencelabs/rust-sbt:$(TOOLCHAIN)
RS_FILE    = $(DIR)/SbtRust.Dockerfile
BUILD_ARG  = --build-arg TOOLCHAIN=$(TOOLCHAIN)

rust-sbt:  ;$(BUILD) -t $(RS_IMG) $(BUILD_ARG) -f $(RS_FILE) .
rs-push:   rust-sbt; docker push $(RS_IMG)

######### Dashboard #########
run-dash:  ;cd dashboard; npm run watch

.PHONY: node node-test worker worker-test dashboard %-bctl-test deploy rust-sbt rs-push
