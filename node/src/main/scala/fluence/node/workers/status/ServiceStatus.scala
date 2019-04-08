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
import fluence.effects.docker.{DockerError, DockerRunning}
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
  docker: Either[DockerError, DockerRunning],
  http: HttpStatus[T]
) {

  /**
   * Returns true iff docker is running, HTTP check performed successfully, and a condition on HTTP result is met
   *
   * @param httpOk Predicate for Http check's status
   */
  def isOk(httpOk: T ⇒ Boolean = _ ⇒ true): Boolean =
    docker.isRight && (http match {
      case HttpCheckStatus(data) ⇒ httpOk(data)
      case _ ⇒ false
    })
}

object ServiceStatus {
  private implicit val dockerRunningEncoder: Encoder[DockerRunning] = deriveEncoder
  private implicit val dockerRunningDecoder: Decoder[DockerRunning] = deriveDecoder

  private implicit val throwableEncoder: Encoder[Throwable] =
    Encoder.encodeString.contramap(e => e.getMessage + Option(e.getCause).map(c => s"; caused by: ${c.getMessage}"))
  private implicit val throwableDecoder: Decoder[Throwable] = Decoder.decodeString.map(new RuntimeException(_))

  private implicit val dockerErrorEncoder: Encoder[DockerError] = deriveEncoder
  private implicit val dockerErrorDecoder: Decoder[DockerError] = deriveDecoder

  private implicit val dockerEitherEncoder: Encoder[Either[DockerError, DockerRunning]] =
    Encoder.encodeEither("failed", "running")
  private implicit val dockerEitherDecoder: Decoder[Either[DockerError, DockerRunning]] =
    Decoder.decodeEither("failed", "running")

  implicit def serviceStatusEncoder[T: Encoder]: Encoder[ServiceStatus[T]] = deriveEncoder

  implicit def serviceStatusDecoder[T: Decoder]: Decoder[ServiceStatus[T]] = deriveDecoder
}
