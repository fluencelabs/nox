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
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.{Decoder, Encoder}

/**
 * Limits on cpu and memory of a container
 *
 * @see [[DockerParams.cpus]], [[DockerParams.memory]], [[DockerParams.memoryReservation]]
 * @param cpus Fraction number of max cores available to a container.
 * @param memoryMb A hard limit on maximum amount of memory available to a container
 * @param memoryReservationMb Amount of memory guaranteed to be allocated for a container
 */
case class DockerLimits(cpus: Option[Double], memoryMb: Option[Int], memoryReservationMb: Option[Int])

object DockerLimits {
  implicit val enc: Encoder[DockerLimits] = deriveEncoder
  implicit val dec: Decoder[DockerLimits] = deriveDecoder
}
