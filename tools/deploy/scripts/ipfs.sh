function start_ipfs()
{
    if [ ! "$(docker ps -q -f name=ipfs)" ]; then
        if [ "$LOCAL_IPFS_ENABLED" == true ]; then
            echo "Starting IPFS container"
            docker-compose --compatibility -f ipfs.yml up -d >/dev/null
            # todo get rid of `sleep`
            sleep 15
            echo "IPFS container is started"
        fi
    fi
}

export FLUENCE_STORAGE="$HOME/.fluence/"

export IPFS_STORAGE="$FLUENCE_STORAGE/ipfs/"

## TODO: run ipfs container