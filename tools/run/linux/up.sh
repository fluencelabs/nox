#!/bin/bash

export DOCKER_IP=$(ifconfig docker0 | grep 'inet ' | awk '{print $2}' | grep -Po "[0-9\.]+")
export HOST_IP=$(ip route get 8.8.8.8 | grep -Po "(?<=src )[0-9\.]+")
docker-compose up -dV
