/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.dataset.grpc.client

import cats.effect.Effect
import monix.eval.{MVar, Task}
import monix.execution.Scheduler

import scala.language.higherKinds

object DatasetClientUtils {

  def run[F[_]: Effect, A](fa: Task[A])(implicit sch: Scheduler): F[A] = fa.to[F]

  /** Returns either client error if present, or server error, or value from server */
  def composeResult[F[_]: Effect](
    clientError: Task[MVar[ClientError]],
    serverErrOrVal: Task[Option[Task[Option[Array[Byte]]]]]
  )(implicit sch: Scheduler): F[Option[Array[Byte]]] =
    run(
      Task.raceMany(
        Seq(
          clientError.flatMap(_.read).flatMap(err ⇒ Task.raiseError(err)), // trying to return occurred clients error
          serverErrOrVal.flatMap {
            // return success result or server error
            case Some(errOrValue) ⇒ errOrValue
            // return occurred clients error
            case None ⇒ clientError.flatMap(_.read).flatMap(err ⇒ Task.raiseError(err))
          }
        )
      )
    )

}
