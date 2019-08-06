package fluence.node.workers.websocket

import io.circe.Encoder
import io.circe.generic.extras.Configuration
import io.circe.generic.extras.semiauto.deriveEncoder

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
    implicit val conf: Configuration = Configuration.default.withDiscriminator("type")
    implicit val websocketResponseDecoder: Encoder[WebsocketResponse] = deriveEncoder[WebsocketResponse]
  }
}
