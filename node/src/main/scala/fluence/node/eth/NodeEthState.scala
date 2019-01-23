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
import scodec.bits.ByteVector
import state.App

/**
 * State of the node, how it's expected to be from Ethereum point of view
 *
 * @param validatorKey Node's validator key
 * @param apps Map of applications to be hosted by the node
 */
case class NodeEthState(
  validatorKey: ByteVector,
  apps: Map[ByteVector, App] = Map.empty
) {

  // TODO should we also use it as a filter for events which are not related to this node but couldn't be checked without the state?
  def advance(event: NodeEthEvent): NodeEthState = event match {
    case RunAppWorker(app) ⇒
      copy(apps = apps + (app.id → app))

    case RemoveAppWorker(appId) ⇒
      copy(apps = apps - appId)

    case DropPeerWorker(appId, `validatorKey`) ⇒
      copy(apps = apps - appId)

    case DropPeerWorker(appId, vk) ⇒
      apps.get(appId).fold(this) { app ⇒
        // Remove the peer the way we do it in smart contract
        val workers = app.cluster.workers
        val i = workers.indexWhere(_.validatorKey == vk)

        val cluster = app.cluster.copy(
          workers = workers.lastOption
          // If this worker is the last one, or if it's not in the list, just filter it out
            .filterNot(_ ⇒ i < 0 || i >= workers.length)
            // Otherwise replace it with the last one, and drop the last
            .fold(workers.filterNot(_.validatorKey === vk))(last ⇒ workers.dropRight(1).updated(i, last))
        )

        // This node still participates in the cluster, so we can safely retain it
        copy(apps = apps.updated(appId, app.copy(cluster = cluster)))
      }

  }
}
