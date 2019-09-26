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

package fluence.effects.testkit

import cats.effect._
import cats.instances.either._
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.monadError._
import org.scalactic.source.Position
import org.scalatest.exceptions.{TestFailedDueToTimeoutException, TestFailedException}
import org.scalatest.time.Span
import org.scalatest.{Timer => _}

import scala.concurrent.TimeoutException
import scala.concurrent.duration._
import scala.language.higherKinds

trait Timed {

  /**
   * Executes `p` every `period` until it either succeeds or `maxWait` timeout passes
   */
  protected def eventually[F[_]: Sync: Timer: Concurrent](
    p: => F[Unit],
    period: FiniteDuration = 1.second,
    maxWait: FiniteDuration = 10.seconds
  )(implicit pos: Position): F[Unit] =
    Concurrent
      .timeout(
        fs2.Stream
          .awakeEvery[F](period)
          .evalMap(_ => Concurrent.timeout(p.attempt, period).attempt.map(_.flatten))
          .takeThrough(_.isLeft) // until p returns Right(Unit)
          .compile
          .last,
        maxWait
      )
      .map {
        case Some(Right(_)) =>
        case Some(Left(e))  => throw e
        case _              => throw new RuntimeException(s"eventually timed out after $maxWait")
      }
      .adaptError(adaptError(pos, maxWait))

  /**
   * Either computes `p` within specified `timeout` or fails the test
   */
  protected def timed[F[_]: Concurrent: Timer](
    p: F[Unit],
    timeout: FiniteDuration = 10.seconds
  )(implicit pos: Position): F[Unit] =
    Concurrent
      .timeout(p.attempt, timeout)
      .attempt
      .map(_.flatten)
      .rethrow
      .adaptError(adaptError(pos, timeout, prefix = ""))

  private def adaptError(
    pos: Position,
    timeout: FiniteDuration,
    prefix: String = "eventually "
  ): PartialFunction[Throwable, Throwable] = {
    def testFailed(cause: Throwable, printCause: Boolean = false) = {
      if (printCause) cause.printStackTrace(System.err)
      new TestFailedDueToTimeoutException(
        _ =>
          Some(
            s"${prefix}timed out after $timeout" + Option(cause.getMessage).filter(_ => printCause).fold("")(": " + _)
          ),
        Some(cause),
        pos,
        None,
        Span.convertDurationToSpan(timeout)
      )
    }

    {
      case e: TestFailedException =>
        e.printStackTrace(System.err)
        e.modifyMessage(m => Some(s"${prefix}timed out after $timeout" + m.fold("")(": " + _)))
      case e: TimeoutException => testFailed(e)
      case e                   => testFailed(e, printCause = true)
    }
  }
}
