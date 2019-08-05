package fluence.node.workers

import cats.Monad
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.log.Log
import cats.syntax.flatMap._
import cats.syntax.either._
import cats.syntax.functor._
import fluence.node.workers.subscription.{OkResponse, PendingResponse, RpcErrorResponse, TimedOutResponse}
import io.circe.{Decoder, Encoder, ParsingFailure}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.parser.parse
import io.circe.syntax._

import scala.language.higherKinds

class WorkersWebsocket[F[_]: Monad: Log](worker: Worker[F], workerApi: WorkerApi) {

  def process(input: WebsocketRequest): F[WebsocketResponse] = {
    input match {
      case TxRequest(tx, id, requestId) =>
        workerApi.sendTx(worker, tx, id).map {
          case Right(data) => TxResponse(requestId, data)
          case Left(error) => ErrorResponse(requestId, error.getMessage)
        }
      case Query(path, data, id, requestId) =>
        workerApi.query(worker, data, path, id).map {
          case Right(data) => QueryResponse(requestId, data)
          case Left(error) => ErrorResponse(requestId, error.getMessage)
        }
      case TxWaitRequest(tx, id, requestId) =>
        workerApi
          .sendTxAwaitResponse(worker, tx, id)
          .map(_.map {
            case OkResponse(_, response)    => TxWaitResponse(requestId, response)
            case RpcErrorResponse(_, error) => ErrorResponse(requestId, error.getMessage)
            case TimedOutResponse(_, tries) => ErrorResponse(requestId, s"Cannot get response after $tries tries")
            case PendingResponse(_)         => ErrorResponse(requestId, s"Unexpected error.")
          })
      case LastManifest(requestId) =>
        workerApi.lastManifest(worker).map(block => LastManifestResponse(requestId, block.map(_.jsonString)))
      case Status(requestId)  => workerApi.status(worker).map(_.map(status => StatusResponse(requestId, status)))
      case P2pPort(requestId) => workerApi.p2pPort(worker).map(port => P2pPortResponse(requestId, port))
    }
  }
}

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

sealed trait WebsocketRequest {
  def requestId: String
}
case class Query(path: String, data: Option[String], id: Option[String], requestId: String) extends WebsocketRequest
case class TxRequest(tx: String, id: Option[String], requestId: String) extends WebsocketRequest
case class TxWaitRequest(tx: String, id: Option[String], requestId: String) extends WebsocketRequest
case class LastManifest(requestId: String) extends WebsocketRequest
case class P2pPort(requestId: String) extends WebsocketRequest
case class Status(requestId: String) extends WebsocketRequest

object WebsocketRequest {
  implicit val websocketRequestDecoder: Decoder[WebsocketRequest] = deriveDecoder[WebsocketRequest]
  implicit val websocketRequestEncoder: Encoder[WebsocketRequest] = deriveEncoder[WebsocketRequest]
  implicit val txEncoder: Encoder[TxRequest] = deriveEncoder[TxRequest]
}

object TestCase extends App {
  import WebsocketRequest._

  /*println(WorkersWebsocket.process("azazaza"))
  val tx: WebsocketRequest = TxRequest("tx data", None)
  val json = tx.asJson.spaces4
  println("json: " + json)*/

}
