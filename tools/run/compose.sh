#!/bin/bash

case "$(uname -s)" in
   Darwin)
     docker-compose -f macos/docker-compose.yml up -dV
     ;;

   Linux)
     (cd linux; ./up.sh)
     ;;
esac