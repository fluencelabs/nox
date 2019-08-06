package fluence.node.workers.websocket

import cats.Monad
import cats.data.EitherT
import fluence.log.Log
import cats.syntax.functor._
import fluence.node.workers.{Worker, WorkerApi}
import fluence.node.workers.subscription.{OkResponse, PendingResponse, RpcErrorResponse, TimedOutResponse}
import io.circe.parser.parse
import io.circe.syntax._

import scala.language.higherKinds

class WorkersWebsocket[F[_]: Monad: Log](worker: Worker[F], workerApi: WorkerApi) {
  import WebsocketRequests._
  import WebsocketResponses._

  def parseAndProcess(input: String): F[String] = {
    val result = for {
      request <- EitherT
        .fromEither(parse(input).flatMap(j => j.as[WebsocketRequest]))
      response <- EitherT.liftF[F, io.circe.Error, WebsocketResponse](process(request))
    } yield {
      response
    }
    result.value.map {
      case Right(v)    => v
      case Left(error) => ErrorResponse("", s"Cannot parse msg. Error: $error, msg: $input")
    }.map(_.asJson.spaces4)
  }

  def process(input: WebsocketRequest): F[WebsocketResponse] = {
    input match {
      case TxRequest(tx, id, requestId) =>
        workerApi.sendTx(worker, tx, id).map {
          case Right(data) => TxResponse(requestId, data)
          case Left(error) => ErrorResponse(requestId, error.getMessage)
        }
      case QueryRequest(path, data, id, requestId) =>
        workerApi.query(worker, data, path, id).map {
          case Right(data) => QueryResponse(requestId, data)
          case Left(error) => ErrorResponse(requestId, error.getMessage)
        }
      case TxWaitRequest(tx, id, requestId) =>
        workerApi
          .sendTxAwaitResponse(worker, tx, id)
          .map {
            case Right(OkResponse(_, response))    => TxWaitResponse(requestId, response)
            case Right(RpcErrorResponse(_, error)) => ErrorResponse(requestId, error.getMessage)
            case Right(TimedOutResponse(_, tries)) =>
              ErrorResponse(requestId, s"Cannot get response after $tries tries")
            case Right(PendingResponse(_)) => ErrorResponse(requestId, s"Unexpected error.")
            case Left(error)               => ErrorResponse(requestId, error.toString)
          }
      case LastManifestRequest(requestId) =>
        workerApi.lastManifest(worker).map(block => LastManifestResponse(requestId, block.map(_.jsonString)))
      case StatusRequest(requestId) =>
        workerApi.status(worker).map {
          case Right(status) => StatusResponse(requestId, status)
          case Left(error)   => ErrorResponse(requestId, error.getMessage)
        }
      case P2pPortRequest(requestId) => workerApi.p2pPort(worker).map(port => P2pPortResponse(requestId, port))
    }
  }
}
