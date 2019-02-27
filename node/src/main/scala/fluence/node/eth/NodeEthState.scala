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

import cats.{Applicative, Monad}
import cats.data.StateT
import fluence.effects.ethclient.data.Block
import scodec.bits.ByteVector
import state.App

import scala.language.higherKinds

/**
 * State of the node, how it's expected to be from Ethereum point of view
 *
 * @param validatorKey Node's validator key
 * @param apps Map of applications to be hosted by the node
 * @param nodesToApps Mapping from node keys to set of application ids, to enable efficient Worker Peers removal
 * @param lastBlock Last block that was seen by node
 */
case class NodeEthState(
  validatorKey: ByteVector,
  apps: Map[Long, App] = Map.empty,
  nodesToApps: Map[ByteVector, Set[Long]] = Map.empty,
  lastBlock: Option[Block] = None
)

object NodeEthState {
  private type State[F[_]] =
    StateT[F, NodeEthState, Seq[NodeEthEvent]]

  private def get[F[_]: Applicative] =
    StateT.get[F, NodeEthState]

  private def set[F[_]: Applicative](state: NodeEthState) =
    StateT.set[F, NodeEthState](state)

  private def pure[F[_]: Applicative](values: NodeEthEvent*): State[F] =
    StateT.pure(values)

  private def modify[F[_]: Applicative](fn: NodeEthState ⇒ NodeEthState) =
    StateT.modify(fn)

  /**
   * Expresses the state change on new block received from the Ethereum network
   */
  def onNewBlock[F[_]: Monad](block: Block): State[F] =
    modify[F](_.copy(lastBlock = Some(block))).map(_ ⇒ Nil)

  // TODO check that it's not yet launched? Handle case if this event reflects chain reorg
  /**
   * Expresses the state change that should be applied on new App deployment event
   */
  def onNodeApp[F[_]: Monad](app: App): State[F] =
    modify[F](
      s ⇒
        s.copy(
          // Add an app
          apps = s.apps.updated(app.id, app),
          // Save the mapping
          nodesToApps = app.cluster.workers.map(_.validatorKey).foldLeft(s.nodesToApps) {
            case (acc, nodeId) ⇒ acc.updated(nodeId, acc.getOrElse(nodeId, Set.empty) + app.id)
          }
      )
    ).map(_ ⇒ RunAppWorker(app) :: Nil)

  /**
   * Expresses the state change that should be applied when an App is deleted
   */
  def onAppDeleted[F[_]: Monad](appId: Long): State[F] =
    get[F].flatMap { s ⇒
      s.apps
        .get(appId)
        .fold(pure()) { app ⇒
          set(
            s.copy(
              // Remove an app
              apps = s.apps - appId,
              // Remove app id from nodesToApps mapping
              nodesToApps = app.cluster.workers.map(_.validatorKey).foldLeft(s.nodesToApps) {
                case (acc, nodeId) ⇒
                  acc.get(nodeId).map(_ - appId).fold(acc) {
                    case appIds if appIds.isEmpty ⇒
                      // It was the last known app for the node
                      acc - nodeId
                    case appIds ⇒
                      acc.updated(nodeId, appIds)
                  }
              }
            )
          ).map(_ ⇒ RemoveAppWorker(appId) :: Nil)
        }
    }

  /**
   * Expresses the state change that should be applied on Node Deleted event
   * @param nodeId Removed Node's ValidatorKey
   */
  def onNodeDeleted[F[_]: Monad](nodeId: ByteVector): State[F] =
    get[F].flatMap {
      case s if s.validatorKey === nodeId ⇒
        // Emptying the node itself
        set(s.copy(apps = Map.empty, nodesToApps = Map.empty))
          .map(_ ⇒ s.apps.keys.map(appId ⇒ RemoveAppWorker(appId)).toSeq)

      case s ⇒
        s.nodesToApps.get(nodeId).fold(pure()) { appIds ⇒
          // Take all the appIds for removed node, and remove it from the workers list of that apps
          val (state, events) =
            appIds // We're collecting the events of removing an app by ourselves, or of dropping the peer of a cluster
              .foldLeft[(NodeEthState, List[NodeEthEvent])]((s.copy(nodesToApps = s.nodesToApps - nodeId), Nil)) {
                case (acc @ (st, evs), appId) ⇒
                  def removeNodeFromApp(app: App): App = {
                    val workers = app.cluster.workers
                    lazy val i = workers.indexWhere(_.validatorKey === nodeId)

                    // Remove the peer the way we do it in smart contract
                    val newWorkers = workers.lastOption match {
                      case Some(last) if last.validatorKey === nodeId =>
                        workers.dropRight(1)

                      case Some(last) if i > 0 =>
                        workers.dropRight(1).updated(i, last)

                      case _ =>
                        workers.filterNot(_.validatorKey === nodeId)
                    }

                    app.copy(cluster = app.cluster.copy(workers = newWorkers))
                  }

                  st.apps
                    .get(appId)
                    .map(removeNodeFromApp)
                    .fold(acc) {
                      case app if app.cluster.workers.isEmpty ⇒
                        // No more workers, drop the app and forget about it
                        (st.copy(apps = st.apps - appId), RemoveAppWorker(appId) :: evs)
                      case app ⇒
                        // Drop peer worker and update the app
                        (st.copy(apps = st.apps.updated(appId, app)), DropPeerWorker(appId, nodeId) :: evs)
                    }
              }
          set(state).map(_ ⇒ events)
        }
    }
}
