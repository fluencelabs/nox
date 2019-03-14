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

package fluence.effects.docker

import java.time.Instant
import java.util.concurrent.{ExecutorService, Executors}

import cats.{~>, Defer, Monad}
import cats.effect._
import cats.syntax.functor._
import cats.syntax.applicativeError._
import fluence.effects.docker.params.DockerParams
import slogging.LazyLogging

import scala.concurrent.ExecutionContext
import scala.language.higherKinds
import scala.sys.process._
import scala.util.control.NonFatal
import scala.util.{Failure, Success, Try}

class DockerIO[F[_]: Monad: LiftIO: ContextShift: Defer](
  ctx: ExecutionContext,
  defaultStopTimeout: Int
) extends LazyLogging {

  private val liftCtx: LiftIO[F] with (IO ~> F) = new LiftIO[F] with (IO ~> F) {
    override def liftIO[A](ioa: IO[A]): F[A] = ContextShift[F].evalOn(ctx)(ioa.to[F])

    override def apply[A](fa: IO[A]): F[A] = liftIO(fa)
  }

  private def getNameIO(containerId: String): IO[String] =
    IO(s"""docker ps -af id=$containerId --format "{{.Names}}" """.!!)
      .map(_.trim.replace("\"", ""))
      .recover {
        case NonFatal(e) ⇒
          logger.warn(s"Error on docker ps: $e")
          ""
      }

  def getName(container: DockerContainer): F[String] =
    getNameIO(container.containerId).to(liftCtx) // TODO should be mapK on EitherT!

  /**
   * Runs a temporary docker container with custom executable. Returns stdout of execution as a string.
   * Caller is responsible for container removal.
   *
   * @param params parameters for Docker container
   * @return a string with execution stdout
   */
  def exec(params: DockerParams.ExecParams): F[String] =
    IO {
      logger.info(s"Executing docker command: ${params.command.mkString(" ")}")
      params.process.!!.trim
    }.to(liftCtx) // TODO should be mapK on EitherT!

  /**
   * Runs a daemonized docker container, providing a single String with the container ID.
   * Calls `docker rm -f` on that ID when stream is over.
   *
   * @param params parameters for Docker container, must start with `docker run -d`
   * @param stopTimeout Container clean up timeout: SIGTERM is sent, and if container is still alive after timeout, SIGKILL produced
   * @return a stream that produces a docker container ID
   */
  def run(params: DockerParams.DaemonParams, stopTimeout: Int = defaultStopTimeout): Resource[F, DockerContainer] =
    Resource
      .makeCase[IO, Try[String]] {
        logger.info(s"Running docker: ${params.command.mkString(" ")}")
        // TODO: if we have another docker container with the same name, we should rm -f it
        IO(Try(params.process.!!).map(_.trim)).map {
          case f @ Failure(err) ⇒
            logger.warn("Cannot run docker container: " + err, err)
            f
          case s ⇒ s
        }
      } {
        case (Success(dockerId), exitCase) ⇒
          getNameIO(dockerId).flatMap { name ⇒
            IO {
              logger.info(s"Going to stop container $name $dockerId, exit case: $exitCase")
              val t = Try(s"docker stop -t $stopTimeout $dockerId".!)
              // TODO should we `docker kill` if Cancel is triggered while stopping?
              logger.debug(s"Stop result: $t")
              t
            }
          }.flatMap {
            case Success(0) ⇒
              getNameIO(dockerId).flatMap { name ⇒
                IO {
                  logger.info(s"Container $dockerId with name $name stopped gracefully, going to rm -v it")
                  val containerLogs = s"docker logs --tail 100 $dockerId".!!.replaceAll("(?m)^", s"$name  ")
                  if (containerLogs.trim.nonEmpty)
                    logger.info(Console.CYAN + containerLogs + Console.RESET)
                  else
                    logger.info(Console.CYAN + s"$name: empty logs." + Console.RESET)
                  s"docker rm -v $dockerId".!
                }.void
              }
            case Failure(err) ⇒
              IO {
                logger.warn(s"Stopping docker container $dockerId errored due to $err, going to rm -v -f it", err)
                s"docker rm -v -f $dockerId".!
              }.void
            case Success(x) ⇒
              IO {
                logger.warn(s"Stopping docker container $dockerId failed, exit code = $x, going to rm -v -f it")
                s"docker rm -v -f $dockerId".!
              }.void
          }.handleError { err ⇒
            logger.error(s"Error cleaning up container $dockerId: $err", err)
            ()
          }
        case (Failure(err), _) ⇒
          logger.warn(s"Cannot cleanup the docker container as it's failed to launch: $err", err)
          IO.unit
      }
      .flatMap[DockerContainer] {
        case Success(dockerId) ⇒
          Resource.pure(DockerContainer(dockerId))
        case Failure(err) ⇒
          logger.warn(s"Resource cannot be acquired, error raised $err", err)
          Resource.liftF[IO, DockerContainer](IO.raiseError(err))
      }
      .mapK(liftCtx)

  /**
   * Inspect the docker container to find out its running status
   *
   * @param container Docker container
   * @return DockerRunStatus
   */
  def checkContainer(container: DockerContainer): F[DockerStatus] = {
    import java.time.format.DateTimeFormatter
    val format = DateTimeFormatter.ISO_DATE_TIME
    val dockerId = container.containerId
    for {
      // TODO move all the errors to value space. Now if $dockerId is unknown, F will be failed
      status ← IO(s"docker inspect -f {{.State.Running}},{{.State.StartedAt}} $dockerId".!!)
      timeIsRunning ← IO {
        val running :: started :: Nil = status.trim.split(',').toList

        // TODO get any reason of why container is stopped
        logger.debug(s"Docker container $dockerId  status = [$running], startedAt = [$started]")

        Instant.from(format.parse(started)).getEpochSecond → running.contains("true")
      }
      (time, isRunning) = timeIsRunning
    } yield
      if (isRunning) DockerRunning(time): DockerStatus
      else DockerStopped(time)
  }.to(liftCtx) // TODO should be mapK on EitherT!

  /**
   *  Run shell command and raise error on non-zero exit code
   */
  private def run(cmd: String): IO[Unit] = {
    IO(cmd.!).flatMap {
      case exit if exit != 0 =>
        logger.error(s"`$cmd` exited with code: $exit")
        IO.raiseError[Unit](new Exception(s"`$cmd` exited with code: $exit"))
      case _ => IO.pure(())
    }
  }

  /**
   *  Create docker network as a resource. Network is deleted after resource is used.
   */
  def makeNetwork(name: String): Resource[F, DockerNetwork] =
    Resource
      .make(run(s"docker network create $name").as(DockerNetwork(name))) {
        case DockerNetwork(n) =>
          IO(logger.info(s"removing network $n"))
            .flatMap(_ => run(s"docker network rm $n"))
      }
      .mapK(liftCtx)

  /**
   * Join (connect to) docker network as a resource. Container will be disconnected from network after resource is used.
   */
  def joinNetwork(container: DockerContainer, network: DockerNetwork): Resource[F, Unit] =
    Resource
      .make(run(s"docker network connect ${network.name} ${container.containerId}"))(
        _ =>
          IO(logger.info(s"disconnecting container ${container.containerId} from network ${network.name} "))
            .flatMap(_ => run(s"docker network disconnect ${network.name} ${container.containerId}"))
      )
      .mapK(liftCtx)
}

object DockerIO extends LazyLogging {

  def apply[F[_]](implicit dio: DockerIO[F]): DockerIO[F] = dio

  def make[F[_]: Monad: LiftIO: ContextShift: Defer](
    ex: ⇒ ExecutorService = Executors.newSingleThreadExecutor(),
    defaultStopTimeout: Int = 10
  ): Resource[F, DockerIO[F]] =
    Resource
      .make(IO(ExecutionContext.fromExecutorService(ex)).to[F])(
        ctx ⇒ IO(ctx.shutdown()).to[F]
      )
      .map(ctx ⇒ new DockerIO[F](ctx, defaultStopTimeout))
}
