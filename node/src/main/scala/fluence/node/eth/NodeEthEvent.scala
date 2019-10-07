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

package fluence.node.eth
import fluence.effects.ethclient.data.Block
import fluence.worker.eth.EthApp
import scodec.bits.ByteVector

sealed trait NodeEthEvent {
  def blockNumber: Option[Long] = None
}

case class RunAppWorker(app: EthApp) extends NodeEthEvent

case class RemoveAppWorker(appId: Long) extends NodeEthEvent

case class DropPeerWorker(appId: Long, validatorKey: ByteVector) extends NodeEthEvent

case class NewBlockReceived(block: Block) extends NodeEthEvent

case object ContractAppsLoaded extends NodeEthEvent
