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

package fluence.node.docker

import cats.Applicative
import cats.syntax.apply._
import cats.syntax.functor._
import cats.effect.{ContextShift, Sync, Timer}
import com.fasterxml.jackson.databind.util.ISO8601DateFormat
import slogging.LazyLogging

import scala.concurrent.duration.FiniteDuration
import scala.util.{Failure, Success, Try}
import scala.language.higherKinds
import scala.sys.process._

object DockerIO extends LazyLogging {

  /**
   * Shifts ''fn'' execution on another thread, and runs it lazily.
   *
   * @param fn the function to run
   */
  private def shiftDelay[F[_]: Sync: ContextShift, A](fn: ⇒ A): F[A] =
    implicitly[ContextShift[F]].shift *> Sync[F].delay(fn)

  /**
   * Runs a docker container, providing a single String with the container ID.
   * Calls `rm -f` on that ID when stream is over.
   *
   * @param params will be concatenated to `docker run -d`
   * @return a stream that produces a docker container ID
   */
  def run[F[_]: Sync: ContextShift](params: DockerParams.Sealed): fs2.Stream[F, String] =
    fs2.Stream.bracketCase {
      logger.info(s"Running docker: ${params.command.mkString(" ")}")
      // TODO: if we have another docker container with the same name, we should rm -f it
      shiftDelay(Try(params.process.!!).map(_.trim()))
    } {
      case (Success(dockerId), exitCase) ⇒
        logger.info(s"Going to cleanup $dockerId, exit case: $exitCase")
        shiftDelay(s"docker rm -f $dockerId".!).map {
          case 0 ⇒ logger.info(s"Container $dockerId successfully removed")
          case x ⇒ logger.warn(s"Stopping docker container $dockerId failed, exit code = $x")
        }
      case (Failure(err), _) ⇒
        logger.warn(s"Can't cleanup the docker container as it's failed to launch: $err", err)
        Applicative[F].unit
    }.flatMap {
      case Success(dockerId) ⇒ fs2.Stream.emit(dockerId)
      case Failure(err) ⇒ fs2.Stream.raiseError(err)
    }

  private val format = new ISO8601DateFormat()

  /**
   * Checks that container is alive every tick.
   *
   * @param period Container will be checked every ''period''
   * @return The pipe that takes a container's ID and returns (timeSinceStart, isAlive) for every ''period'' of time
   */
  def check[F[_]: Timer: Sync: ContextShift](
    period: FiniteDuration
  ): fs2.Pipe[F, String, (Long, Boolean)] =
    _.flatMap(
      dockerId ⇒
        fs2.Stream
          .awakeEvery[F](period)
          .evalMap(
            d ⇒
              shiftDelay(s"docker inspect -f {{.State.Running}},{{.State.StartedAt}} $dockerId".!!).map { status ⇒
                val running :: started :: Nil = status.trim.split(',').toList

                logger.debug(s"Docker container $dockerId  status = [$running], startedAt = [$started]")
                val now = System.currentTimeMillis()
                val time = format.parse(started).getTime
                (now - time) → status.contains("true")
            }
        )
    )
}
