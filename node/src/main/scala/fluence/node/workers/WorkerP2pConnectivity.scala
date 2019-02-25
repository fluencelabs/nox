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

import cats.{Applicative, Parallel}
import cats.syntax.applicativeError._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.effect.{Concurrent, Fiber, Sync, Timer}
import cats.instances.vector._
import com.softwaremill.sttp.{SttpBackend, sttp, _}
import fluence.ethclient.EthRetryPolicy
import fluence.node.eth.state.WorkerPeer
import slogging.LazyLogging

import scala.language.higherKinds
import scala.concurrent.duration._
import scala.util.Try

object WorkerP2pConnectivity extends LazyLogging {

  // TODO don't use Eth retry policy; add jitter
  private def retryUntilSuccess[F[_]: Timer: Sync, T](fn: F[T], retryPolicy: EthRetryPolicy): F[T] =
    fn.attempt.flatMap {
      case Right(v) ⇒ Applicative[F].pure(v)
      case Left(err) ⇒
        err.printStackTrace()
        logger.debug(s"Got error $err, retrying")
        Timer[F].sleep(retryPolicy.delayPeriod) *> retryUntilSuccess(fn, retryPolicy.next)
    }

  def join[F[_]: Concurrent: Timer, G[_]](
    worker: Worker[F],
    peers: Vector[WorkerPeer],
    retryPolicy: EthRetryPolicy = EthRetryPolicy(1.second, 10.seconds)
  )(
    implicit P: Parallel[F, G],
    sttpBackend: SttpBackend[F, Nothing]
  ): F[Fiber[F, Unit]] =
    Concurrent[F].start(
      Parallel.parTraverse_(peers) { p ⇒
        logger.debug(Console.RED + s"Peer address: ${p.ip.getHostAddress}:${p.apiPort}" + Console.RESET)

        val getPort: F[Short] = sttp
          .get(uri"http://${p.ip.getHostAddress}:${p.apiPort}/apps/${worker.appId}/p2pPort")
          .send()
          .flatMap { resp ⇒
            Sync[F].fromEither(
              resp.body.left
                .map(new RuntimeException(_))
                .flatMap(v ⇒ Try(v.toShort).toEither)
            )
          }

        retryUntilSuccess(getPort, retryPolicy).flatMap { p2pPort ⇒
          logger.info(Console.BLUE + s"GOT PEER p2p PORT: ${p.peerAddress(p2pPort)}" + Console.RESET)
          retryUntilSuccess(
            worker.tendermint
              .unsafeDialPeers(p.peerAddress(p2pPort) :: Nil, persistent = true)
              .value
              .flatMap { res ⇒
                logger.info(Console.YELLOW + s"UNSAFE DIAL PEERS REPLIED $res" + Console.RESET)
                Sync[F].fromEither(res)
              },
            retryPolicy
          )
        }
      }
    )

}
