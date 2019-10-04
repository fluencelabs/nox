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

package fluence.effects.tendermint.rpc.response

import java.util.Base64

import cats.syntax.either._
import cats.instances.either._
import cats.syntax.flatMap._
import io.circe.parser._
import io.circe.syntax._
import io.circe.{Decoder, DecodingFailure, Encoder, HCursor}
import scodec.bits.ByteVector

private[tendermint] case class Base64Json[T](value: T)

object Base64Json {

  private def base64ToString(str: String) =
    ByteVector
      .fromBase64Descriptive(str)
      .leftMap(e ⇒ DecodingFailure(s"Error decoding to base64 from $str: $e", List.empty))
      .flatMap(
        _.decodeUtf8.leftMap(e ⇒ DecodingFailure(s"Error decoding $str base64 => bytes => utf8: $e", List.empty))
      )

  private def strToJson(str: String) =
    parse(str).leftMap(DecodingFailure.fromThrowable(_, List.empty))

  implicit def decodeResponse[T: Decoder]: Decoder[Base64Json[T]] =
    (c: HCursor) => (c.as[String] >>= base64ToString >>= strToJson >>= (_.as[T])).map(Base64Json(_))

  implicit def encodeResponse[T: Encoder]: Encoder[Base64Json[T]] =
    (b64: Base64Json[T]) => Base64.getEncoder.encodeToString(b64.value.asJson.noSpaces.getBytes).asJson
}
