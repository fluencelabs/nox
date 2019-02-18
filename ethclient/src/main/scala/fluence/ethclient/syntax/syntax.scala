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

package fluence.ethclient
import cats.{Applicative, Monad}
import cats.data.EitherT
import cats.effect._
import cats.syntax.flatMap._
import cats.syntax.apply._
import cats.syntax.applicativeError._
import io.reactivex.Flowable
import slogging.LazyLogging
import fs2.interop.reactivestreams._
import fluence.ethclient.helpers.JavaFutureConversion._
import org.web3j.protocol.core.RemoteCall

import scala.concurrent.duration._
import scala.language.higherKinds
import scala.language.existentials
import scala.util.control.NonFatal

package object syntax {

  /**
   * Provides block-until-success method to get result from EthClient's request, dropping away failures and retrying infinitely.
   *
   * @param request Request, converted to EitherT with EthClient
   * @tparam F Effect, Timer is used for backoff
   * @tparam T Response
   */
  implicit class EitherTRequestOps[F[_]: Timer: Monad, T](request: EitherT[F, E, T] forSome {
    type E <: EthRequestError
  }) extends LazyLogging {

    def retryUntilSuccess(implicit ethRetryPolicy: EthRetryPolicy = EthRetryPolicy.Default): F[T] =
      request.value.flatMap {
        case Right(value) ⇒ Applicative[F].pure(value)
        case Left(err) ⇒
          logger.warn(
            s"EthClient request failed, going to retry after ${ethRetryPolicy.delayPeriod.toSeconds} seconds",
            err
          )
          Timer[F].sleep(ethRetryPolicy.delayPeriod) *> retryUntilSuccess(ethRetryPolicy.next)
      }
  }

  /**
   * Safe conversion from rx Flowable to fs2 Stream: rx is initiated again in case of any error.
   *
   * @param flowable RX Flowable
   * @tparam T Value type
   */
  implicit class FlowableToStreamOps[T](flowable: Flowable[T]) extends LazyLogging {

    def toStreamRetrying[F[_]: ConcurrentEffect: Timer](
      onErrorRetryAfter: FiniteDuration = EthRetryPolicy.Default.delayPeriod
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
     * @param ethRetryPolicy Retry policy
     */
    def callUntilSuccess[F[_]: LiftIO: Timer: Monad](
      implicit ethRetryPolicy: EthRetryPolicy = EthRetryPolicy.Default
    ): F[T] =
      call[F].retryUntilSuccess

  }
}
