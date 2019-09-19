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

import java.util.Base64

import io.circe.{Decoder, Encoder, Json}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import scodec.bits.ByteVector

import scala.util.Try

/**
 * A structure for aggregating data specific to building `Query` ABCI method response.
 *
 * @param height height corresponding to state for which result given
 * @param result requested result, if found
 * @param code response code
 * @param info response message
 */
case class QueryResponse(height: Long, result: Array[Byte], code: QueryCode.Value, info: String) {

  // TODO make correct json
  def toResponseString(id: String = "dontcare"): String =
    s"""
       | {
       |   "jsonrpc": "2.0",
       |   "id": "$id",
       |   "result": {
       |    "code": ${code.id},
       |    "response": {
       |      "info": "$info",
       |      "value": "${ByteVector(result).toBase64}",
       |      "height": "$height"
       |    }
       |   }
       | }
           """.stripMargin

  /*
{
  "jsonrpc": "2.0",
  "id": "dontcare",
  "result": {
    "response": {
      "info": "Responded for path srDhIRCxA6ux/0",
      "value": "XzAKMjQuOTM5MzkzOTM5MzkzOTM4",
      "height": "103"
    }
  }
}
 */
}

object QueryResponse {
  implicit val byteEncoder: Encoder[Array[Byte]] =
    Encoder[String].contramap((b: Array[Byte]) => ByteVector.view(b).toBase64)

  implicit val byteDecoder: Decoder[Array[Byte]] =
    Decoder[String].emap(ByteVector.fromBase64Descriptive(_)).map(_.toArray)

  implicit val encoder: Encoder[QueryResponse] = deriveEncoder
  implicit val decoder: Decoder[QueryResponse] = deriveDecoder
}
