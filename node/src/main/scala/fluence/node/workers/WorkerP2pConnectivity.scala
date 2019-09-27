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

package fluence.node.workers

import cats.Parallel
import cats.data.EitherT
import cats.effect.{Concurrent, Fiber, Resource, Timer}
import com.softwaremill.sttp._
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.instances.vector._
import fluence.effects.sttp.syntax._
import fluence.bp.api.DialPeers
import fluence.effects.sttp.SttpEffect
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.worker.eth.WorkerPeer

import scala.util.Try
import scala.language.higherKinds

/**
 * Connects a worker to other peers of the cluster
 */
object WorkerP2pConnectivity {

  /**
   * Ping peers to get theirs p2p port for the app, then pass that port to Worker's TendermintRPC to dial.
   *
   * @param appId Id of the application for the cluster we're joining
   * @param dialPeers Tendermint RPC of the current app
   * @param peers All the other peers to form the cluster
   * @param backoff Retry policy for exponential backoff in reties
   * @tparam F Concurrent to make a fiber so that you can cancel the joining job, Timer to make retries
   * @return Fiber for concurrent job of inquiring peers and putting their addresses to Tendermint
   */
  def join[F[_]: Concurrent: Timer: SttpEffect: Parallel](
    appId: Long,
    dialPeers: DialPeers[F],
    peers: Vector[WorkerPeer],
    backoff: Backoff[EffectError] = Backoff.default
  )(
    implicit log: Log[F]
  ): F[Fiber[F, Unit]] =
    Log[F].scope("p2p-join") { implicit log: Log[F] =>
      Concurrent[F].start(
        Parallel.parTraverse_(peers) { p ⇒
          // Get p2p port for an app
          val getPort: EitherT[F, EffectError, Short] = sttp
            .get(uri"http://${p.ip.getHostAddress}:${p.apiPort}/apps/$appId/p2pPort")
            .send()
            .decodeBody(v ⇒ Try(v.toShort).toEither)
            .leftMap[EffectError](identity)

          Log[F].debug(s"Peer API address: ${p.ip.getHostAddress}:${p.apiPort}") >>
            // Get p2p port, pass it to worker's tendermint
            backoff(getPort).flatMap { p2pPort ⇒
              Log[F].trace(s"Got Peer p2p port: ${p.peerAddress(p2pPort)}") >>
                backoff(
                  dialPeers
                    .dialPeers(p.peerAddress(p2pPort) :: Nil)
                )
            }
        }
      )
    }

  /**
   * Ping peers to get theirs p2p port for the app, then pass that port to Worker's TendermintRPC to dial.
   * Works in background until all peers responded. Stops the background job on resource release.
   *
   * @param appId Id of the application for the cluster we're joining
   * @param dialPeers Tendermint RPC of the current app
   * @param peers All the other peers to form the cluster
   * @param backoff Retry policy for exponential backoff in reties
   * @tparam F Concurrent to make a fiber so that you can cancel the joining job, Timer to make retries
   */
  def make[F[_]: Concurrent: Timer: Log: SttpEffect: Parallel](
    appId: Long,
    dialPeers: DialPeers[F],
    peers: Vector[WorkerPeer],
    backoff: Backoff[EffectError] = Backoff.default
  ): Resource[F, Unit] =
    Resource.make(join(appId, dialPeers, peers, backoff))(_.cancel).void

}
