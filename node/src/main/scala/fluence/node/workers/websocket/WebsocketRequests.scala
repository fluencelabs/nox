package fluence.node.workers.websocket

import io.circe.{Decoder, Encoder}
import io.circe.generic.extras.Configuration
import io.circe.generic.extras.semiauto.{deriveDecoder, deriveEncoder}

object WebsocketRequests {
  sealed trait WebsocketRequest {
    def requestId: String
  }
  case class QueryRequest(path: String, data: Option[String], id: Option[String], requestId: String)
      extends WebsocketRequest
  case class TxRequest(tx: String, id: Option[String], requestId: String) extends WebsocketRequest
  case class TxWaitRequest(tx: String, id: Option[String], requestId: String) extends WebsocketRequest
  case class LastManifestRequest(requestId: String) extends WebsocketRequest
  case class P2pPortRequest(requestId: String) extends WebsocketRequest
  case class StatusRequest(requestId: String) extends WebsocketRequest

  object WebsocketRequest {
    implicit val conf: Configuration = Configuration.default.withDiscriminator("type").withSnakeCaseConstructorNames

    implicit val websocketRequestDecoder: Decoder[WebsocketRequest] = deriveDecoder[WebsocketRequest]
    implicit val websocketRequestEncoder: Encoder[WebsocketRequest] = deriveEncoder[WebsocketRequest]
  }
}
