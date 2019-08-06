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
import cats.data.EitherT
import cats.syntax.apply._
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.applicative._
import io.circe.parser._
import io.circe.syntax._
import cats.effect.{Concurrent, Sync}
import fluence.effects.tendermint.rpc.http.{
  RpcBlockParsingFailed,
  RpcBodyMalformed,
  RpcError,
  RpcRequestErrored,
  RpcRequestFailed
}
import fluence.log.{Log, LogFactory}
import fluence.node.workers.subscription.{
  OkResponse,
  PendingResponse,
  RpcErrorResponse,
  RpcTxAwaitError,
  TendermintResponseError,
  TimedOutResponse,
  TxInvalidError,
  TxParsingError
}
import fs2.concurrent.Queue
import io.circe.Decoder
import org.http4s.dsl.Http4sDsl
import org.http4s.server.websocket.WebSocketBuilder
import org.http4s.websocket.WebSocketFrame
import org.http4s.websocket.WebSocketFrame.Text
import org.http4s.{HttpRoutes, Response}

import scala.concurrent.duration._
import scala.language.higherKinds

object WorkersHttp {

  /**
   * Encodes a tendermint response to HTTP format.
   *
   */
  def tendermintResponseToHttp[F[_]: Monad](
    appId: Long,
    response: Either[RpcError, String]
  )(implicit log: Log[F], dsl: Http4sDsl[F]): F[Response[F]] = {
    import dsl._
    response match {
      case Right(result) ⇒
        log.trace(s"RPC responding with OK: $result") *>
          Ok(result)

      case Left(err) ⇒
        rpcErrorToResponse(err)
    }
  }

  /**
   * Encodes errors to HTTP format.
   *
   */
  def rpcErrorToResponse[F[_]: Monad](error: RpcError)(implicit log: Log[F], dsl: Http4sDsl[F]): F[Response[F]] = {
    import dsl._
    error match {
      case RpcRequestFailed(err) ⇒
        log.warn(s"RPC request failed", err) *>
          InternalServerError(err.getMessage)

      case err: RpcRequestErrored ⇒
        log.warn(s"RPC request errored", err) *>
          InternalServerError(err.error)

      case RpcBodyMalformed(err) ⇒
        log.warn(s"RPC body malformed: $err", err)
        BadRequest(err.getMessage)

      case err: RpcBlockParsingFailed =>
        log.warn(s"RPC $err", err)
        InternalServerError(err.getMessage)
    }
  }

  /**
   * Routes for Workers API.
   *
   * @param pool Workers pool to get workers from
   * @param dsl Http4s DSL to build routes with
   */
  def routes[F[_]: Sync: LogFactory](pool: WorkersPool[F], workerApi: WorkerApi)(
    implicit dsl: Http4sDsl[F],
    F: Concurrent[F]
  ): HttpRoutes[F] = {
    import dsl._
    import WebsocketRequest._
    import WebsocketResponse._

    object QueryPath extends QueryParamDecoderMatcher[String]("path")
    object QueryData extends OptionalQueryParamDecoderMatcher[String]("data")
    object QueryId extends OptionalQueryParamDecoderMatcher[String]("id")

    def withWorker(appId: Long)(fn: Worker[F] => F[Response[F]])(implicit log: Log[F]): F[Response[F]] = {
      pool.get(appId).flatMap {
        case None =>
          log.debug(s"RPC Requested app $appId, but there's no such worker in the pool") *>
            NotFound("App not found on the node")
        case Some(worker) =>
          fn(worker)
      }
    }

    // Routes comes there
    HttpRoutes.of {
      case GET -> Root / LongVar(appId) / "ws" =>
        LogFactory[F].init("http" -> "websocket", "app" -> appId.toString) >>= { implicit log =>
          withWorker(appId)(w => {
            val websocket = new WorkersWebsocket(w, workerApi)
            val echoReply: fs2.Pipe[F, WebSocketFrame, WebSocketFrame] =
              _.evalMap {
                case Text(msg, _) =>
                  val respF = for {
                    request <- EitherT
                      .fromEither(parse(msg).flatMap(j => j.as[WebsocketRequest]))
                    response <- EitherT.liftF[F, io.circe.Error, WebsocketResponse](websocket.process(request))
                  } yield {
                    response
                  }
                  respF.value.map {
                    case Left(err) => Text(err.getMessage)
                    case Right(ok) => Text(ok.asJson.spaces4)
                  }
                case _ => Text("Something new").pure[F]
              }

            Queue
              .unbounded[F, WebSocketFrame]
              .flatMap { q =>
                val d = q.dequeue.through(echoReply)
                val e = q.enqueue
                WebSocketBuilder[F].build(d, e)
              }
          })
        }

      case GET -> Root / LongVar(appId) / "query" :? QueryPath(path) +& QueryData(data) +& QueryId(id) ⇒
        LogFactory[F].init("http" -> "query", "app" -> appId.toString) >>= { implicit log =>
          withWorker(appId)(w => workerApi.query(w, data, path, id).flatMap(tendermintResponseToHttp(appId, _)))
        }

      case GET -> Root / LongVar(appId) / "status" ⇒
        LogFactory[F].init("http" -> "status", "app" -> appId.toString) >>= { implicit log =>
          withWorker(appId)(w => workerApi.status(w).flatMap(tendermintResponseToHttp(appId, _)))
        }

      case GET -> Root / LongVar(appId) / "p2pPort" ⇒
        LogFactory[F].init("http" -> "p2pPort", "app" -> appId.toString) >>= { implicit log =>
          log.debug(s"Worker p2pPort") *>
            withWorker(appId)(workerApi.p2pPort(_).map(_.toString).flatMap(Ok(_)))
        }

      case GET -> Root / LongVar(appId) / "lastManifest" ⇒
        LogFactory[F].init("http" -> "lastManifest", "app" -> appId.toString) >>= { implicit log =>
          // TODO try to get last manifest from local Kademlia storage
          withWorker(appId)(
            w =>
              workerApi.lastManifest(w).flatMap {
                case Some(m) ⇒ Ok(m.jsonString)
                case None ⇒
                  log.debug("There's no available manifest yet") *>
                    NoContent()
            }
          )
        }

      case req @ POST -> Root / LongVar(appId) / "tx" :? QueryId(id) ⇒
        LogFactory[F].init("http" -> "tx", "app" -> appId.toString) >>= { implicit log =>
          req.decode[String] { tx ⇒
            withWorker(appId)(w => workerApi.sendTx(w, tx, id).flatMap(tendermintResponseToHttp(appId, _)))
          }
        }

      case req @ POST -> Root / LongVar(appId) / "txWaitResponse" :? QueryId(id) ⇒
        LogFactory[F].init("http" -> "txAwaitResponse", "app" -> appId.toString) >>= { implicit log =>
          req.decode[String] { tx ⇒
            withWorker(appId)(
              w =>
                workerApi.sendTxAwaitResponse(w, tx, id).flatMap {
                  case Right(queryResponse) =>
                    queryResponse match {
                      case OkResponse(_, response) =>
                        Ok(response)
                      case RpcErrorResponse(_, r) => rpcErrorToResponse(r)
                      case TimedOutResponse(id, tries) =>
                        RequestTimeout(
                          s"Request $id couldn't be processed after $tries blocks. Try later or start a new session to continue."
                        )
                      case PendingResponse(_) =>
                        InternalServerError("PendingResponse is returned. Unexpected error.")
                    }
                  case Left(err) =>
                    err match {
                      // return an error from tendermint as is to the client
                      case TendermintResponseError(response) => Ok(response)
                      case RpcTxAwaitError(rpcError)         => rpcErrorToResponse(rpcError)
                      case TxParsingError(msg, _)            => BadRequest(msg)
                      case TxInvalidError(msg)               => InternalServerError(msg)
                    }
              }
            )
          }
        }
    }
  }
}
