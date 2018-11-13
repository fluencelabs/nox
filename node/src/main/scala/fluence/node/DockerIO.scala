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
import cats.Applicative
import cats.effect._
import cats.syntax.apply._
import cats.syntax.functor._
import slogging.LazyLogging

import scala.concurrent.duration.FiniteDuration
import scala.sys.process._
import scala.language.higherKinds
import scala.util.{Failure, Success, Try}

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
   * @param whatToRun will be concatenated to `docker run -d`
   * @return a stream that produces a docker container ID
   */
  def run[F[_]: Sync: ContextShift](whatToRun: String): fs2.Stream[F, String] =
    fs2.Stream.bracketCase {
      logger.info(s"Running docker: $whatToRun")
      shiftDelay(Try(s"docker run -d $whatToRun".!!).map(_.trim()))
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

  /**
   * Checks that container is alive every tick.
   *
   * @param period Container will be checked every ''period''
   * @return The pipe that takes a container's ID and returns (timeSinceStart, isAlive) for every ''period'' of time
   */
  def check[F[_]: Timer: Sync: ContextShift](
    period: FiniteDuration
  ): fs2.Pipe[F, String, (FiniteDuration, Boolean)] =
    _.flatMap(
      dockerId ⇒
        fs2.Stream
          .awakeEvery[F](period)
          .evalMap(
            d ⇒
              shiftDelay(s"docker inspect -f '{{.State.Running}}' $dockerId".!!).map { status ⇒
                logger.debug(s"Docker container $dockerId  status = [$status]")
                d → status.contains("true")
            }
        )
    )
}
