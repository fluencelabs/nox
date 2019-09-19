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

package fluence.statemachine.api.query

import io.circe.{Decoder, Encoder}

import scala.util.Try

object QueryCode extends Enumeration {
  val Ok, CannotParseHeader, Dropped, NotFound, Pending = Value

  implicit val decoder: Decoder[QueryCode.Value] =
    Decoder[Int].emap(i => Try(QueryCode(i)).toEither.left.map(e => s"Error decoding query code: $e"))
  implicit val encoder: Encoder[QueryCode.Value] = Encoder[Int].contramap(_.id)
}
