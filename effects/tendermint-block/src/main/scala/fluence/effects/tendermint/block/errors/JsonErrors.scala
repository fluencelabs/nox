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

package fluence.effects.tendermint.block.errors

import fluence.effects.WithCause
import io.circe.{DecodingFailure, ParsingFailure}
import scalapb_json.JsonFormatException

trait JsonErrors {
  case class FixBytesError(msg: String) extends TendermintBlockError

  case class JsonDecodingError private (cause: DecodingFailure)
      extends TendermintBlockError with WithCause[DecodingFailure]

  case class JsonParsingError private (cause: ParsingFailure)
      extends TendermintBlockError with WithCause[ParsingFailure]

  trait ProtobufJsonError extends TendermintBlockError

  case class ProtobufJsonFormatError(cause: JsonFormatException)
      extends ProtobufJsonError with WithCause[JsonFormatException]

  case class ProtobufJsonUnknownError(cause: Throwable) extends ProtobufJsonError with WithCause[Throwable]

  implicit object LiftDecodingFailure extends ConvertError[DecodingFailure, JsonDecodingError](JsonDecodingError.apply)
  implicit object LiftParsingFailure extends ConvertError[ParsingFailure, JsonParsingError](JsonParsingError.apply)
  implicit object LiftProtobufJsonError
      extends ConvertError[Throwable, ProtobufJsonError]({
        case c: JsonFormatException => ProtobufJsonFormatError(c)
        case c: Throwable => ProtobufJsonUnknownError(c)
      })

}
