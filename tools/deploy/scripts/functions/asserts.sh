function check_fluence_installed()
{
    command -v $PWD/fluence >/dev/null 2>&1 || command -v fluence >/dev/null 2>&1 || {
        echo >&2 "Can't find fluence in PATH or in $PWD"
        exit 1
    }
}

function check_envs()
{
    if [ -n "$PROD_DEPLOY" ]; then
        declare -p CONTRACT_ADDRESS &>/dev/null || {
            echo >&2 "CONTRACT_ADDRESS is not defined"
            exit 1
        }
    fi

    declare -p OWNER_ADDRESS &>/dev/null || {
        echo >&2 "OWNER_ADDRESS is not defined"
        exit 1
    }
    declare -p API_PORT &>/dev/null || {
        echo >&2 "API_PORT is not defined"
        exit 1
    }
    declare -p CAPACITY &>/dev/null || {
        echo >&2 "CAPACITY is not defined"
        exit 1
    }
    declare -p PARITY_RESERVED_PEERS &>/dev/null || {
        echo >&2 "PARITY_RESERVED_PEERS is not defined"
        exit 1
    }
    declare -p NAME &>/dev/null || {
        echo >&2 "NAME is not defined"
        exit 1
    }
    declare -p HOST_IP &>/dev/null || {
        echo >&2 "HOST_IP is not defined"
        exit 1
    }
    declare -p SWARM_ADDRESS &>/dev/null || {
        echo >&2 "SWARM_ADDRESS is not defined"
        exit 1
    }
    declare -p IPFS_ADDRESS &>/dev/null || {
        echo >&2 "IPFS_ADDRESS is not defined"
        exit 1
    }
    declare -p REMOTE_STORAGE_ENABLED &>/dev/null || {
        echo >&2 "REMOTE_STORAGE_ENABLED is not defined"
        exit 1
    }

    declare -p ETHEREUM_IP &>/dev/null || {
        echo >&2 "ETHEREUM_IP is not defined"
        exit 1
    }
    declare -p ETHEREUM_ADDRESS &>/dev/null || {
        echo >&2 "ETHEREUM_ADDRESS is not defined"
        exit 1
    }
    declare -p ETHEREUM_SERVICE &>/dev/null || {
        echo >&2 "ETHEREUM_SERVICE is not defined"
        exit 1
    }
    declare -p IMAGE_TAG &>/dev/null || {
        echo >&2 "IMAGE_TAG is not defined"
        exit 1
    }
}
