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

package fluence.node.workers

import cats.Monad
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.log.Log
import cats.syntax.flatMap._
import cats.syntax.either._
import io.circe.generic.semiauto._
import io.circe._
import cats.syntax.functor._
import fluence.node.workers.subscription.{OkResponse, PendingResponse, RpcErrorResponse, TimedOutResponse}
import io.circe.generic.extras.{Configuration, ConfiguredJsonCodec}
import io.circe.{Decoder, Encoder}
import io.circe.generic.extras.semiauto.{deriveDecoder, deriveEncoder}
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
