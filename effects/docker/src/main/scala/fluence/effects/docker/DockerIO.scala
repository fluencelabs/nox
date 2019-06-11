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

import cats.data.EitherT
import cats.{~>, Applicative, Defer, Monad}
import cats.effect._
import cats.syntax.functor._
import cats.syntax.apply._
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import fluence.effects.docker.params.DockerParams
import fluence.log.Log

import scala.concurrent.ExecutionContext
import scala.language.higherKinds
import scala.sys.process._
import scala.util.{Failure, Success, Try}

class DockerIO[F[_]: Monad: LiftIO: ContextShift: Defer](
  ctx: ExecutionContext,
  defaultStopTimeout: Int
) {

  private val liftCtx: IO ~> F = new (IO ~> F) {
    override def apply[A](fa: IO[A]): F[A] = ContextShift[F].evalOn(ctx)(fa.to[F])
  }

  /**
   * Run shell command
   */
  private def runShell(cmd: String)(implicit log: Log[F]): EitherT[F, DockerError, String] =
    Log.eitherT[F, DockerError].info(s"Running Docker command: `$cmd`") *>
      IO(
        cmd.!!.trim
      ).attemptT
        .mapK(liftCtx)
        .leftMap(DockerCommandError(cmd, _))

  /**
   *  Run shell command and raise error on non-zero exit code. Command response is dropped
   */
  private def runShellVoid(cmd: String)(implicit log: Log[F]): F[Unit] =
    IO(cmd.!).to[F].flatMap {
      case exit if exit != 0 =>
        Log[F].error(s"`$cmd` exited with code: $exit") *>
          IO.raiseError[Unit](new Exception(s"`$cmd` exited with code: $exit")).to[F]
      case _ =>
        Applicative[F].unit
    }

  /**
   * Get Docker container's name, if it's possible
   *
   * @param container Docker Container
   */
  def getName(container: DockerContainer)(implicit log: Log[F]): EitherT[F, DockerError, String] =
    runShell(s"""docker ps -af id=${container.containerId} --format "{{.Names}}" """)
      .map(_.trim.replace("\"", ""))

  /**
   * Runs a temporary docker container with custom executable. Returns stdout of execution as a string.
   * Caller is responsible for container removal.
   *
   * @param params parameters for Docker container
   * @return a string with execution stdout
   */
  def exec(params: DockerParams.ExecParams)(implicit log: Log[F]): EitherT[F, DockerError, String] =
    Log.eitherT[F, DockerError].info(s"Executing docker command: ${params.command.mkString(" ")}") *>
      IO(params.process.!!)
        .map(_.trim)
        .attemptT
        .leftMap[DockerError](DockerCommandError(params.command.mkString(" "), _))
        .mapK(liftCtx)

  /**
   * Runs a daemonized docker container, providing a single String with the container ID.
   * Calls `docker rm -f` on that ID when stream is over.
   *
   * @param params parameters for Docker container, must start with `docker run -d`
   * @param stopTimeout Container clean up timeout: SIGTERM is sent, and if container is still alive after timeout, SIGKILL produced
   * @return a stream that produces a docker container ID
   */
  def run(params: DockerParams.DaemonParams, stopTimeout: Int = defaultStopTimeout)(
    implicit log: Log[F]
  ): Resource[F, DockerContainer] = {
    val runContainer: F[Either[Throwable, String]] =
      Log[F].info(s"Running docker daemon: ${params.command.mkString(" ")}") *>
        IO(params.process.!!)
          .map(_.trim)
          .attemptT
          .mapK(liftCtx)
          .leftSemiflatMap { err ⇒
            Log[F].warn("Cannot run docker container: " + err, err) as err
          }
          .value

    def tryStopContainer(name: String, dockerId: String, exitCase: ExitCase[Throwable]): F[Try[Int]] =
      Log[F].info(s"Going to stop container $name $dockerId, exit case: $exitCase") >>
        liftCtx(IO(Try(s"docker stop -t $stopTimeout $dockerId".!))) >>=
        (t ⇒ Log[F].debug(s"Stop result: $t").as(t)) // TODO should we `docker kill` if Cancel is triggered while stopping?

    def rmOnGracefulStop(name: String, dockerId: String): F[Unit] =
      Log[F].info(s"Container $dockerId with name $name stopped gracefully, going to rm -v it") >>
        liftCtx(IO(s"docker logs --tail 100 $dockerId".!!.replaceAll("(?m)^", s"$name  "))).flatMap { containerLogs ⇒
          if (containerLogs.trim.nonEmpty)
            Log[F].info(Console.CYAN + containerLogs + Console.RESET)
          else
            Log[F].info(Console.CYAN + s"$name: empty logs." + Console.RESET)
        } >>
        liftCtx(IO(s"docker rm -v $dockerId".!).void)

    def forceRmWhenCannotStop(name: String, dockerId: String, err: Throwable): F[Unit] =
      Log[F].warn(s"Stopping docker container $name $dockerId errored due to $err, going to rm -v -f it", err) >>
        liftCtx(IO(s"docker rm -v -f $dockerId".!).void)

    def forceRmWhenStopNonZero(name: String, dockerId: String, code: Int): F[Unit] =
      Log[F].warn(s"Stopping docker container $name $dockerId failed, exit code = $code, going to rm -v -f it") >>
        liftCtx(IO(s"docker rm -v -f $dockerId".!).void)

    Resource
      .makeCase(runContainer) {
        case (Right(dockerId), exitCase) ⇒
          getName(DockerContainer(dockerId))
            .getOrElse("(name is unknown)")
            .flatMap { name ⇒
              tryStopContainer(name, dockerId, exitCase).flatMap {
                case Success(0) ⇒
                  rmOnGracefulStop(name, dockerId)

                case Failure(err) ⇒
                  forceRmWhenCannotStop(name, dockerId, err)

                case Success(x) ⇒
                  forceRmWhenStopNonZero(name, dockerId, x)
              }
            }

        case (Left(err), _) ⇒
          Log[F].warn(s"Cannot cleanup the docker container as it's failed to launch: $err", err)
      }
      .flatMap[DockerContainer] {
        case Right(dockerId) ⇒
          Resource.pure(DockerContainer(dockerId))
        case Left(err) ⇒
          Log.resource[F].warn(s"Resource cannot be acquired, error raised $err", err) *>
            Resource.liftF(IO.raiseError[DockerContainer](err).to[F])
      }
  }

  /**
   * Inspect the docker container to find out its running status
   *
   * @param container Docker container
   * @return DockerRunning or any error found on the way
   */
  def checkContainer(container: DockerContainer)(implicit log: Log[F]): EitherT[F, DockerError, DockerRunning] = {
    import java.time.format.DateTimeFormatter
    val format = DateTimeFormatter.ISO_DATE_TIME
    val dockerId = container.containerId
    for {
      status ← runShell(s"docker inspect -f {{.State.Running}},{{.State.StartedAt}} $dockerId")
      timeIsRunning ← IO {
        val running :: started :: Nil = status.trim.split(',').toList

        Instant.from(format.parse(started)).getEpochSecond → running.contains("true")
      }.attemptT
        .mapK(liftCtx)
        .flatTap {
          case (time, running) ⇒
            // TODO get any reason of why container is stopped
            Log.eitherT[F, Throwable].debug(s"Docker container $dockerId  status = [$running], time = [$time]")
        }
        .leftMap(DockerException(s"Cannot parse container status: $status", _): DockerError)
    } yield timeIsRunning
  }.subflatMap {
    case (time, true) ⇒ Right(DockerRunning(time))
    case (time, false) ⇒ Left(DockerContainerStopped(time))
  }

  /**
   *  Create docker network as a resource. Network is deleted after resource is used.
   */
  def makeNetwork(name: String)(implicit log: Log[F]): Resource[F, DockerNetwork] =
    Resource
      .make(runShellVoid(s"docker network create $name").as(DockerNetwork(name))) {
        case DockerNetwork(n) =>
          Log[F].info(s"removing network $n") >>
            runShellVoid(s"docker network rm $n")
      }

  /**
   * Join (connect to) docker network as a resource. Container will be disconnected from network after resource is used.
   */
  def joinNetwork(container: DockerContainer, network: DockerNetwork)(implicit log: Log[F]): Resource[F, Unit] =
    Resource
      .make(runShellVoid(s"docker network connect ${network.name} ${container.containerId}"))(
        _ =>
          Log[F].info(s"disconnecting container ${container.containerId} from network ${network.name} ")
            >> runShellVoid(s"docker network disconnect ${network.name} ${container.containerId}")
      )

}

object DockerIO {

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
