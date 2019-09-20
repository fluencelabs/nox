# Copyright 2018 Fluence Labs Limited
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

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
