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

package fluence.worker.api

import cats.data.EitherT

import scala.language.higherKinds
import shapeless._

trait WorkerContext[F[_]] {
  def appId: Long

  type Components <: HList

  type W <: Worker[F]

  // should be a flag: not prepared, in progress, ready, failed to prepare
  def isPrepared: F[Boolean]
  // launches preparation on background
  def startPreparation(): F[Unit]
  // blocks until prepared, may fail
  def prepare(): F[Unit]

  // so we have several components here:
  // - local p2p port allocation
  // - peer connectivity: at least to discover peer ports
  // - tendermint configs
  // - downloaded code
  //
  // Later on, we do more with a ready worker:
  // - Launch block uploading (??? or is it bundled -- yes it is bundled within BlockProducer, which should be built upon StateMachine)
  // - Subscribe for blocks to provide pubsub and request-response patterns
  //
  // all components must be prepared on demand and retain in prepared state
  // components could be cached (e.g. no need to re-init tendermint)
  // when context is deallocated, components could be cached, resources released (?)
  // if this app is deleted, all resources are removed
  //
  // Worker pool should contain both contexts and workers (?).
  // When worker is launching, context must be already prepared.
  // When context is stopped or removed, worker is also stopped.
  // So worker actually is inside the context! Context's boundaries must be applied to the worker!
  // How to express it?
  // At the same time, Worker depends on Context
  // Things like block uploading, etc are parts of worker's context as well, actually. So that's a lifecycle of
  // worker: once worker is started for the first time, some worker-dependent services should also start.
  // All of this could be expressed in types.
  //
  // Components -> Worker -> Components
  // It also matters how things are closed

  def worker: EitherT[F, *, W]

  trait Component {
    def init()

    def prepare()

    def onWorker()

  }
}
