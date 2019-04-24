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

package fluence.effects.tendermint.block

import io.circe.{DecodingFailure, ParsingFailure}

import scala.util.Either
import scala.util.control.NoStackTrace

case class ConvertError[E, EE](convert: E => EE) {
  def apply(e: E): EE = convert(e)
}

trait TendermintBlockError extends NoStackTrace

trait WithCause[E <: Throwable] extends TendermintBlockError {
  def cause: E

  initCause(cause)
}

case class FixBytesError(msg: String) extends TendermintBlockError

case class JsonDecodingError private (cause: DecodingFailure) extends WithCause[DecodingFailure]

object JsonDecodingError {
  implicit object LiftDecodingFailure extends ConvertError[DecodingFailure, JsonDecodingError](apply)
}

case class JsonParsingError private (cause: ParsingFailure) extends WithCause[ParsingFailure]

object JsonParsingError {
  implicit object LiftParsingFailure extends ConvertError[ParsingFailure, JsonParsingError](apply)
}

object ErrorHandling {
  import cats.syntax.either._

  implicit class TrivialHandlerOps[A, E, EE](either: Either[E, A]) {
    def convertError(implicit convert: ConvertError[E, EE]): Either[EE, A] = either.leftMap(convert(_))
//    def flatMap[A1](f: A => Either[E, A1])(implicit convert: ConvertError[E, EE]): Either[EE, A1] = {
//      either.right.flatMap(f(_)).leftMap(convert(_))
//    }
  }
}
