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
import cats.effect.{ContextShift, Resource, Sync, Timer}
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import com.fasterxml.jackson.databind.util.ISO8601DateFormat
import slogging.LazyLogging

import scala.concurrent.duration._
import scala.language.higherKinds
import scala.sys.process._
import scala.util.{Failure, Success, Try}

/**
 * Docker container IO wrapper
 *
 * @param containerId Running Container ID
 */
case class DockerIO(containerId: String) {

  /**
   * Performs a `docker inspect` command for this container
   */
  def check[F[_]: Sync: ContextShift]: F[DockerRunStatus] =
    DockerIO.checkContainer(containerId)

  /**
   * Performs `docker inspect`, fetching container's status, periodically
   *
   * @param period Period to check
   */
  def checkPeriodically[F[_]: Timer: Sync: ContextShift](
    period: FiniteDuration
  ): fs2.Stream[F, DockerRunStatus] =
    fs2.Stream.emit(containerId) through DockerIO.checkPeriodically(period)
}

object DockerIO extends LazyLogging {

  /**
   * Shifts ''fn'' execution on another thread, and runs it lazily.
   *
   * @param fn the function to run
   */
  private def shiftDelay[F[_]: Sync: ContextShift, A](fn: ⇒ A): F[A] =
    ContextShift[F].shift *> Sync[F].delay(fn)

  /**
   * Runs a temporary docker container with custom executable. Returns stdout of execution as a string.
   * Caller is responsible for container removal.
   *
   * @param params parameters for Docker container
   * @return a stream with execution stdout
   */
  def exec[F[_]: Sync: ContextShift](
    params: DockerParams.ExecParams
  ): F[String] =
    shiftDelay {
      logger.info(s"Executing docker command: ${params.command.mkString(" ")}")
      params.process.!!.trim
    }

  /**
   * Runs a daemonized docker container, providing a single String with the container ID.
   * Calls `docker rm -f` on that ID when stream is over.
   *
   * @param params parameters for Docker container, must start with `docker run -d`
   * @return a stream that produces a docker container ID
   */
  def run[F[_]: Sync: ContextShift](params: DockerParams.DaemonParams): Resource[F, DockerIO] =
    Resource.makeCase {
      logger.info(s"Running docker: ${params.command.mkString(" ")}")
      // TODO: if we have another docker container with the same name, we should rm -f it
      shiftDelay(Try(params.process.!!).map(_.trim)).map {
        case f @ Failure(err) ⇒
          logger.warn("Cannot run docker container: " + err, err)
          f
        case s ⇒ s
      }
    } {
      case (Success(dockerId), exitCase) ⇒
        shiftDelay {
          logger.info(s"Going to cleanup $dockerId, exit case: $exitCase")
          s"docker stop $dockerId".!
        }.flatMap {
          case 0 ⇒
            shiftDelay {
              logger.info(s"Container $dockerId stopped gracefully, going to rm -v it")
              logger.info(Console.CYAN + s"docker logs $dockerId".!! + Console.RESET)
              s"docker rm -v $dockerId".!
            }
          case x ⇒
            shiftDelay {
              logger.warn(s"Stopping docker container $dockerId failed, exit code = $x, going to rm -v -f it")
              s"docker rm -v -f $dockerId".!
            }
        }
      case (Failure(err), _) ⇒
        logger.warn(s"Cannot cleanup the docker container as it's failed to launch: $err", err)
        Applicative[F].unit
    }.flatMap {
      case Success(dockerId) ⇒ Resource.pure(DockerIO(dockerId))
      case Failure(err) ⇒ Resource.liftF(Sync[F].raiseError(err))
    }

  /**
   * Checks that container is alive every tick.
   *
   * @param period Container will be checked every ''period''
   * @return The pipe that takes a container's ID and returns (timeSinceStart, isAlive) for every ''period'' of time
   */
  def checkPeriodically[F[_]: Timer: Sync: ContextShift](
    period: FiniteDuration
  ): fs2.Pipe[F, String, DockerRunStatus] =
    _.flatMap(
      dockerId ⇒
        fs2.Stream
          .awakeEvery[F](period)
          .evalMap(
            _ ⇒ checkContainer(dockerId)
        )
    )

  /**
   * Inspect the docker container to find out its running status
   *
   * @param dockerId Container ID
   * @tparam F Effect, monadic error is possible
   * @return DockerRunStatus
   */
  def checkContainer[F[_]: Sync: ContextShift](dockerId: String): F[DockerRunStatus] = {
    val format = new ISO8601DateFormat()
    for {
      status ← shiftDelay(s"docker inspect -f {{.State.Running}},{{.State.StartedAt}} $dockerId".!!)
      timeIsRunning ← Sync[F].catchNonFatal {
        val running :: started :: Nil = status.trim.split(',').toList

        logger.debug(s"Docker container $dockerId  status = [$running], startedAt = [$started]")

        format.parse(started).getTime → running.contains("true")
      }
      (time, isRunning) = timeIsRunning
    } yield DockerRunStatus(time, isRunning)
  }
}
