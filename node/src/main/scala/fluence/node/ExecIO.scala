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

package fluence.node
import cats.{Applicative, ApplicativeError}
import cats.effect._
import cats.syntax.apply._
import cats.syntax.functor._

import scala.sys.process._
import scala.language.higherKinds
import scala.util.{Failure, Success, Try}

/**
 * Functional wrapper for a console command
 * @param cmd Command with its arguments
 */
case class ExecIO(cmd: String*) {

  /**
   * Run the command with F effect, capture its output.
   *
   * @tparam F The effect
   */
  def run[F[_]: Sync: ContextShift]: F[Try[String]] =
    implicitly[ContextShift[F]].shift *> Sync[F].delay(Try(cmd.!!))

}

object ExecIO {
  // run a process, capture its output (docker container id)
  // for health, ask docker if container is alive, then ping inside for a healthcheck
  // to stop, run a command with container id

  def shiftDelay[F[_]: Sync: ContextShift, A](fn: ⇒ A): F[A] =
    implicitly[ContextShift[F]].shift *> Sync[F].delay(fn)

  def docker[F[_]: Sync: ContextShift](whatInside: String): Resource[F, String] =
    Resource
      .make(
        shiftDelay(Try(s"docker run $whatInside".!!))
      ) {
        case Success(dockerId) ⇒ shiftDelay(s"docker rm -f $dockerId".!).map(_ ⇒ ())
        case Failure(_) ⇒ Applicative[F].unit
      }
      .flatMap[String] { dockerTry ⇒
        ApplicativeError[Resource[F, ?], Throwable].fromTry(dockerTry)
      }

}
