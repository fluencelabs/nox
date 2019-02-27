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

package fluence.effects.ethclient
import cats.Monad
import cats.data.EitherT
import cats.effect._
import cats.syntax.applicativeError._
import fluence.effects.Backoff
import io.reactivex.Flowable
import slogging.LazyLogging
import fs2.interop.reactivestreams._
import fluence.effects.ethclient.helpers.JavaFutureConversion._
import org.web3j.protocol.core.RemoteCall

import scala.concurrent.duration._
import scala.language.higherKinds
import scala.util.control.NonFatal

package object syntax {

  /**
   * Safe conversion from rx Flowable to fs2 Stream: rx is initiated again in case of any error.
   *
   * @param flowable RX Flowable
   * @tparam T Value type
   */
  implicit class FlowableToStreamOps[T](flowable: Flowable[T]) extends LazyLogging {

    def toStreamRetrying[F[_]: ConcurrentEffect: Timer](
      onErrorRetryAfter: FiniteDuration = Backoff.default.delayPeriod
    ): fs2.Stream[F, T] =
      flowable.toStream[F]().handleErrorWith {
        case NonFatal(err) ⇒
          logger.error(s"Flowable.toStreamRetrying errored with $err", err)
          toStreamRetrying(onErrorRetryAfter).delayBy(onErrorRetryAfter)
      }
  }

  implicit class RichRemoteCall[T](remoteCall: RemoteCall[T]) {
    import EthClient.ioToF

    /**
     * Call sendAsync on RemoteCall and delay it's side effects
     *
     * @tparam F effect
     * @return Result of the call
     */
    def call[F[_]: LiftIO]: EitherT[F, EthRequestError, T] =
      IO.suspend(remoteCall.sendAsync().asAsync[IO]).attemptT.leftMap(EthRequestError).mapK(ioToF)

    /**
     * Call sendAsync on RemoteCall, repeat until successful result is received
     *
     * This methid may block forever.
     *
     * @param backoff Retry policy
     */
    def callUntilSuccess[F[_]: LiftIO: Timer: Monad](
      implicit backoff: Backoff[EthRequestError] = Backoff.default
    ): F[T] =
      backoff(call[F])
  }
}
