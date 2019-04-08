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

import cats.data.EitherT
import cats.Parallel
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.effect.{Concurrent, Fiber, Timer}
import cats.instances.vector._
import com.softwaremill.sttp.{SttpBackend, sttp, _}
import fluence.effects.{Backoff, EffectError}
import fluence.node.eth.state.WorkerPeer
import slogging.LazyLogging

import scala.language.higherKinds
import scala.util.Try

/**
 * Connects a worker to other peers of the cluster
 */
object WorkerP2pConnectivity extends LazyLogging {

  /**
   * Ping peers to get theirs p2p port for the app, then pass that port to Worker's TendermintRPC to dial.
   *
   * @param worker Local worker, App cluster participant
   * @param peers All the other peers to form the cluster
   * @param backoff Retry policy for exponential backoff in reties
   * @param P Parallelize request to pings
   * @param sttpBackend Used to perform http requests
   * @tparam F Concurrent to make a fiber so that you can cancel the joining job, Timer to make retries
   * @tparam G F.Par
   * @return Fiber for concurrent job of inquiring peers and putting their addresses to Tendermint
   */
  def join[F[_]: Concurrent: Timer, G[_]](
    worker: Worker[F],
    peers: Vector[WorkerPeer],
    backoff: Backoff[EffectError] = Backoff.default
  )(
    implicit P: Parallel[F, G],
    sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing]
  ): F[Fiber[F, Unit]] =
    Concurrent[F].start(
      Parallel.parTraverse_(peers) { p ⇒
        logger.debug(s"Peer API address: ${p.ip.getHostAddress}:${p.apiPort}")

        // Get p2p port for an app
        val getPort: EitherT[F, EffectError, Short] = sttp
          .get(uri"http://${p.ip.getHostAddress}:${p.apiPort}/apps/${worker.appId}/p2pPort")
          .send()
          .leftMap(new RuntimeException(_) with EffectError)
          .flatMap { resp ⇒
            EitherT.fromEither(
              // Awful
              resp.body.left
                .map(new RuntimeException(_) with EffectError)
                .flatMap(v ⇒ Try(v.toShort).toEither.left.map(new RuntimeException(_) with EffectError))
            )
          }

        // Get p2p port, pass it to worker's tendermint
        backoff(getPort).flatMap { p2pPort ⇒
          logger.debug(s"Got Peer p2p port: ${p.peerAddress(p2pPort)}")

          backoff(
            EitherT(
              worker.withServices(_.tendermint)(
                _.unsafeDialPeers(p.peerAddress(p2pPort) :: Nil, persistent = true).value.map { res ⇒
                  logger.debug(s"dial_peers replied: $res")
                  res
                }
              )
            )
          )
        }
      }
    )

}
