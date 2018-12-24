#!/bin/bash

case "$(uname -s)" in
   Darwin)
     export DOCKER_IP=host.docker.internal
     export HOST_IP=host.docker.internal
     docker-compose up -dV
     ;;

   Linux)
     export DOCKER_IP=$(ifconfig docker0 | grep 'inet ' | awk '{print $2}' | grep -Po "[0-9\.]+")
     export HOST_IP=$(ip route get 8.8.8.8 | grep -Po "(?<=src )[0-9\.]+")
     docker-compose up -dV
     ;;
esac
