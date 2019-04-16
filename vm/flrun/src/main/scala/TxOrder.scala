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

import cats.{ApplicativeError, Monad, MonadError}
import cats.effect.concurrent.Ref
import cats.effect.{ContextShift, Timer}
import cats.syntax.applicative._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._

import scala.concurrent.duration._
import scala.language.higherKinds

case class TxOrder[F[_]: ContextShift: Timer](txIds: Ref[F, Map[String, Int]])(implicit F: MonadError[F, Throwable])
    extends slogging.LazyLogging {

  /**
   * Wait until `id` becomes next to latest processed request
   */
  def waitOrder(id: TxId): F[Unit] = {
    for {
      last <- txIds.get.map(_.getOrElse(id.session, -1))
      nextToLast = id.count - last == 1
      _ <- if (nextToLast) {
        ().pure[F]
      } else {
        if (last == -1) logger.warn(s"First request should start with counter = 0, was ${id.count} (${id.session})")

        if (id.count <= last) {
          F.raiseError[Unit](
            new RuntimeException(s"Counter should be bigger than $last, was ${id.count} (${id.session})")
          )
        } else {
          ContextShift[F].shift *> Timer[F].sleep(100.millis) *> waitOrder(id)
        }
      }
    } yield ()
  }

  def set(id: TxId): F[Unit] =
    for {
      _ <- txIds.modify(m => {
        val last = m.getOrElse(id.session, -1)
        val nextToLast = id.count - last == 1
        if (nextToLast)
          m.updated(id.session, id.count) -> Unit
        else
          m -> Unit
      })
    } yield ()
}
