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

package fluence.node.workers.websocket

import io.circe.{Decoder, Encoder}
import io.circe.generic.extras.Configuration
import io.circe.generic.extras.semiauto.{deriveDecoder, deriveEncoder}

object WebsocketResponses {
  sealed trait WebsocketResponse {
    def requestId: String
  }
  case class ErrorResponse(requestId: String, error: String) extends WebsocketResponse
  case class QueryResponse(requestId: String, data: String) extends WebsocketResponse
  case class TxResponse(requestId: String, data: String) extends WebsocketResponse
  case class TxWaitResponse(requestId: String, data: String) extends WebsocketResponse
  case class LastManifestResponse(requestId: String, lastManifest: Option[String]) extends WebsocketResponse
  case class P2pPortResponse(requestId: String, p2pPort: Short) extends WebsocketResponse
  case class StatusResponse(requestId: String, status: String) extends WebsocketResponse

  object WebsocketResponse {
    implicit val conf: Configuration =
      Configuration.default.withDiscriminator("type").withSnakeCaseConstructorNames.withSnakeCaseMemberNames
    implicit val websocketResponseEncoder: Encoder[WebsocketResponse] = deriveEncoder[WebsocketResponse]
    implicit val websocketResponseDecoder: Decoder[WebsocketResponse] = deriveDecoder[WebsocketResponse]
  }
}
