#!/bin/bash
# CAUTION: this script would KILL and REMOVE all existing Docker containers and REMOVE all non-default Docker networks

docker kill $(docker ps -q)
docker rm $(docker ps -q -a)
docker network rm $(docker network ls -q)
