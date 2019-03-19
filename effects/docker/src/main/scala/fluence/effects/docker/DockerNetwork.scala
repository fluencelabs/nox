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
import cats.effect.Resource

import scala.language.higherKinds

/**
 * Represents docker network, defined by it's name
 */
case class DockerNetwork(name: String) extends AnyVal

object DockerNetwork extends slogging.LazyLogging {

  /**
   *  Create docker network as a resource. Network is deleted after resource is used.
   */
  def make[F[_]: DockerIO](name: String): Resource[F, DockerNetwork] =
    DockerIO[F].makeNetwork(name)

  /**
   * Join (connect to) docker network as a resource. Container will be disconnected from network after resource is used.
   */
  def join[F[_]: DockerIO](container: DockerContainer, network: DockerNetwork): Resource[F, Unit] =
    DockerIO[F].joinNetwork(container, network)
}
