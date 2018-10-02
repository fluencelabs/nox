docker kill $(docker ps -q)
docker rm $(docker ps -q -a)
docker network rm $(docker network ls -q)
