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

package fluence.node.workers.status
import fluence.effects.docker.DockerStatus
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto._

/**
 * Represents a (dockerised) service status
 *
 * @param docker Docker container status
 * @param http Http status for the service running in that container
 * @tparam T Custom status info type
 */
case class ServiceStatus[+T](
  docker: DockerStatus,
  http: HttpStatus[T]
) {

  /**
   * Returns true iff docker is running, HTTP check performed successfully, and a condition on HTTP result is met
   *
   * @param httpOk Predicate for Http check's status
   */
  def isOk(httpOk: T ⇒ Boolean = _ ⇒ true): Boolean =
    docker.isRunning && (http match {
      case HttpCheckStatus(data) ⇒ httpOk(data)
      case _ ⇒ false
    })
}

object ServiceStatus {
  implicit val dockerStatusEncoder: Encoder[DockerStatus] = deriveEncoder
  implicit val dockerStatusDecoder: Decoder[DockerStatus] = deriveDecoder

  implicit def serviceStatusEncoder[T: Encoder]: Encoder[ServiceStatus[T]] = deriveEncoder

  implicit def serviceStatusDecoder[T: Decoder]: Decoder[ServiceStatus[T]] = deriveDecoder
}
