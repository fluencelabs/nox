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
import cats.effect.{ContextShift, Resource, Sync}

import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.node.docker.DockerIO.shiftDelay

import scala.language.higherKinds
import scala.sys.process._

// Represents docker network, defined by it's name
case class DockerNetwork(name: String) extends AnyVal

object DockerNetwork extends slogging.LazyLogging {

  // Run shell command and raise error on non-zero exit code
  private def run[F[_]: ContextShift](cmd: String)(implicit F: Sync[F]): F[Unit] = {
    shiftDelay(cmd.!).flatMap {
      case exit if exit != 0 =>
        logger.error(s"`$cmd` exited with code: $exit")
        F.raiseError[Unit](new Exception(s"`$cmd` exited with code: $exit"))
      case _ => F.pure(())
    }
  }

  // Create docker network as a resource. Network is deleted after resource is used.
  def make[F[_]: ContextShift](name: String)(implicit F: Sync[F]): Resource[F, DockerNetwork] =
    Resource.make(run(s"docker network create $name").as(DockerNetwork(name))) {
      case DockerNetwork(n) =>
        F.delay(logger.info(s"removing network $n"))
          .flatMap(_ => run(s"docker network rm $n"))
    }

  // Join (connect to) docker network as a resource. Container will be disconnected from network after resource is used.
  def join[F[_]: ContextShift](container: String, network: DockerNetwork)(
    implicit F: Sync[F]
  ): Resource[F, Unit] =
    Resource.make(run(s"docker network connect ${network.name} $container"))(
      _ =>
        F.delay(logger.info(s"disconnecting container $container from network ${network.name} "))
          .flatMap(_ => run(s"docker network disconnect ${network.name} $container"))
    )
}
