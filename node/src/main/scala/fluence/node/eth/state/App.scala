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

package fluence.node.eth.state

import org.web3j.abi.datatypes.generated._
import scodec.bits.ByteVector

/**
 * Represents an App deployed to some cluster
 *
 * @param id Application ID as defined in Fluence contract
 * @param storageHash Hash of the code in Swarm
 * @param cluster A cluster that hosts this App
 */
case class App private[eth] (
  id: ByteVector,
  storageHash: ByteVector,
  cluster: Cluster
) {
  lazy val appIdHex: String = id.dropWhile(_ == 0).toHex
}

object App {

  private[eth] def apply(appId: Bytes32, storageHash: Bytes32, cluster: Cluster): App =
    App(ByteVector(appId.getValue), ByteVector(storageHash.getValue), cluster)

}
