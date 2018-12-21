#!/bin/bash

case "$(uname -s)" in
   Darwin)
     docker-compose -f docker-compose.yml kill
     ;;

   Linux)
     docker-compose -f docker-compose.yml kill
     ;;
esac

pkill -f ganache

docker ps -a | grep fluence | awk '{ print $1 }' | xargs docker rm -f

docker volume prune -f
