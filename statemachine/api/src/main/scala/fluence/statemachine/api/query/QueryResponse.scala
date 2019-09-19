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

import io.circe.syntax._
import io.circe.{Decoder, Encoder, HCursor, Json}
import scodec.bits.ByteVector

/**
 * A structure for aggregating data specific to building `Query` ABCI method response.
 *
 * @param height height corresponding to state for which result given
 * @param result requested result, if found
 * @param code response code
 * @param info response message
 */
case class QueryResponse(height: Long, result: Array[Byte], code: QueryCode.Value, info: String) {
  def toResponseString(id: String = "dontcare"): String = (id, this).asJson.spaces2
}

object QueryResponse {
  implicit val byteEncoder: Encoder[Array[Byte]] =
    Encoder[String].contramap((b: Array[Byte]) => ByteVector.view(b).toBase64)

  implicit val byteDecoder: Decoder[Array[Byte]] =
    Decoder[String].emap(ByteVector.fromBase64Descriptive(_)).map(_.toArray)

  implicit val encoder: Encoder[(String, QueryResponse)] = {
    case (id: String, resp: QueryResponse) =>
      Json.obj(
        ("jsonrpc", Json.fromString("2.0")),
        ("id", Json.fromString(id)),
        ("result", Json.obj {
          (
            "response",
            Json.obj(
              ("code", Json.fromInt(resp.code.id)),
              ("info", Json.fromString(resp.info)),
              ("height", Json.fromString(resp.height.toString)),
              ("value", resp.result.asJson)
            )
          )
        })
      )
  }

  implicit val decoder: Decoder[QueryResponse] = (c: HCursor) => {
    val res = c.downField("result").downField("response")
    for {
      // Tendermint seems to omit Ok code from json
      code <- res.getOrElse[QueryCode.Value]("code")(QueryCode.Ok)
      info <- res.downField("info").as[String]
      height <- res.downField("height").as[Long]
      value <- res.downField("value").as[Array[Byte]]
    } yield QueryResponse(height, value, code, info)
  }
}

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

{
"jsonrpc": "2.0",
"id": "dontcare",
"result": {
  "response": {
    "code": 4,
    "info": "Transaction is not yet processed: gOULkkG5B46y/3",
    "height": "109"
  }
}
}
 */
