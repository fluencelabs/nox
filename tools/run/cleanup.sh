#!/bin/bash

docker-compose -f docker-compose.yml kill

pkill -f ganache

docker ps -a | grep fluence | awk '{ print $1 }' | xargs docker rm -f

docker volume prune -f
