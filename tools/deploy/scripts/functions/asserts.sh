function check_envs()
{
    declare -p CONTRACT_ADDRESS &>/dev/null || {
        echo >&2 "CONTRACT_ADDRESS is not defined"
        exit 1
    }
    declare -p OWNER_ADDRESS &>/dev/null || {
        echo >&2 "OWNER_ADDRESS is not defined"
        exit 1
    }
    declare -p API_PORT &>/dev/null || {
        echo >&2 "API_PORT is not defined"
        exit 1
    }
    declare -p HOST_IP &>/dev/null || {
        echo >&2 "HOST_IP is not defined"
        exit 1
    }
    declare -p IPFS_ADDRESS &>/dev/null || {
        echo >&2 "IPFS_ADDRESS is not defined"
        exit 1
    }
    declare -p ETHEREUM_IP &>/dev/null || {
        echo >&2 "ETHEREUM_IP is not defined"
        exit 1
    }
}
