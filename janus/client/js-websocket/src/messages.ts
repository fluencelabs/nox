/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

interface Relay {
    action: string,
    peer_id: string,
    data: string,
    p_key: string,
    signature: string
}

export function genMessage(peerId: string, data: string, p_key: string, sign: string): Relay {
    return {
        action: "Relay",
        peer_id: peerId,
        data: data,
        p_key: p_key,
        signature: sign
    }
}

interface GetNetworkState {
    action: string
}

const getNetworkState: GetNetworkState = {
    action: "GetNetworkState"
};

export function networkStateMessage(): GetNetworkState {
  return getNetworkState;
}
