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
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}

import scala.util.control.NoStackTrace

/**
 * Result of Http check: not performed, check failed, or status of type T is fetched
 *
 * @tparam T Success check's data type
 */
sealed trait HttpStatus[+T]

/**
 * Request hasn't been made for some external reason, e.g. service is known to be not launched
 */
case class HttpCheckNotPerformed(reason: String) extends HttpStatus[Nothing]

/**
 * Request has been made, but response contains failure
 *
 * @param cause Cause of failure
 */
case class HttpCheckFailed(cause: Throwable) extends HttpStatus[Nothing]

/**
 * Request has been made, response received
 *
 * @param data Response value
 * @tparam T Success check's data type
 */
case class HttpCheckStatus[+T](data: T) extends HttpStatus[T]

object HttpStatus {
  private implicit val encodeThrowable: Encoder[Throwable] = Encoder[String].contramap(_.getLocalizedMessage)

  private implicit val decodeThrowable: Decoder[Throwable] =
    Decoder[String].map(s => new Exception(s) with NoStackTrace)

  implicit def httpStatusEncoder[T: Encoder]: Encoder[HttpStatus[T]] = deriveEncoder
  implicit def httpStatusDecoder[T: Decoder]: Decoder[HttpStatus[T]] = deriveDecoder
}
