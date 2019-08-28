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
import cats.syntax.apply._
import cats.syntax.functor._
import cats.syntax.flatMap._
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
  TendermintResponseDeserializationError,
  TimedOutResponse,
  TxInvalidError,
  TxParsingError
}
import fluence.node.workers.websocket.WorkersWebsocket
import fs2.concurrent.Queue
import fluence.statemachine.data.Tx
import org.http4s.dsl.Http4sDsl
import org.http4s.server.websocket.WebSocketBuilder
import org.http4s.websocket.WebSocketFrame
import org.http4s.websocket.WebSocketFrame.Text
import org.http4s.{HttpRoutes, Response}
import io.circe.syntax._

import scala.concurrent.duration._
import scala.language.higherKinds

object WorkersHttp {

  /**
   * Encodes a tendermint response to HTTP format.
   *
   */
  private def tendermintResponseToHttp[F[_]: Monad](
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
  private def rpcErrorToResponse[F[_]: Monad](error: RpcError)(implicit log: Log[F],
                                                               dsl: Http4sDsl[F]): F[Response[F]] = {
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
  def routes[F[_]: Sync: LogFactory: Concurrent](pool: WorkersPool[F], workerApi: WorkerApi)(
    implicit dsl: Http4sDsl[F]
  ): HttpRoutes[F] = {
    import dsl._

    object QueryPath extends QueryParamDecoderMatcher[String]("path")
    object QueryData extends OptionalQueryParamDecoderMatcher[String]("data")
    object QueryId extends OptionalQueryParamDecoderMatcher[String]("id")
    object QueryWait extends OptionalQueryParamDecoderMatcher[Int]("wait")

    def withWorker(appId: Long)(fn: Worker[F] => F[Response[F]])(implicit log: Log[F]): F[Response[F]] =
      pool.get(appId).flatMap {
        case None =>
          log.debug(s"RPC Requested app $appId, but there's no such worker in the pool") *>
            NotFound("App not found on the node")
        case Some(worker) =>
          fn(worker)
      }

    // Routes comes there
    HttpRoutes.of {
      case GET -> Root / LongVar(appId) / "ws" =>
        LogFactory[F].init("http" -> "websocket", "app" -> appId.toString) >>= { implicit log =>
          withWorker(appId)(w => {
            val websocket = new WorkersWebsocket(w, workerApi)
            val processMessages: fs2.Pipe[F, WebSocketFrame, WebSocketFrame] =
              _.evalMap {
                case Text(msg, _) =>
                  websocket.processRequest(msg).map(Text(_))
                case m => log.error(s"Unsupported message: $m") as Text("Unsupported")
              }

            Queue
              .unbounded[F, WebSocketFrame]
              .flatMap { q =>
                val d = q.dequeue.through(processMessages)
                val e = q.enqueue
                WebSocketBuilder[F].build(d, e)
              }
          })
        }

      case GET -> Root / LongVar(appId) / "query" :? QueryPath(path) +& QueryData(data) +& QueryId(id) ⇒
        LogFactory[F].init("http" -> "query", "app" -> appId.toString) >>= { implicit log =>
          withWorker(appId)(w => workerApi.query(w, data, path, id).flatMap(tendermintResponseToHttp(appId, _)))
        }

      case GET -> Root / LongVar(appId) / "status" :? QueryWait(wait) ⇒
        LogFactory[F].init("http" -> "status", "app" -> appId.toString) >>= { implicit log =>
          // Fetches the worker's status, waiting no more than 10 seconds (if ?wait=$SECONDS is provided), or 1 second otherwise
          withWorker(appId)(
            _.services
              .status(wait.filter(_ < 10).fold(1.second)(_.seconds))
              .flatMap(st ⇒ Ok(st.asJson.noSpaces))
          )
        }

      case GET -> Root / LongVar(appId) / "status" / "tendermint" ⇒
        LogFactory[F].init("http" -> "status/tendermint", "app" -> appId.toString) >>= { implicit log =>
          withWorker(appId)(w => workerApi.tendermintStatus(w).flatMap(tendermintResponseToHttp(appId, _)))
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
            withWorker(appId)(
              w =>
                workerApi
                  .sendTx(w, tx, id)
                  .flatTap(r => log.debug(s"tx.head: ${tx.takeWhile(_ != '\n')}"))
                  .flatMap(tendermintResponseToHttp(appId, _))
            )
          }
        }

      case req @ POST -> Root / LongVar(appId) / "txWaitResponse" :? QueryId(id) ⇒
        LogFactory[F].init("http" -> "txAwaitResponse", "app" -> appId.toString) >>= { log =>
          req.decode[String] { tx ⇒
            val txHead = Tx.splitTx(tx.getBytes).fold("not parsed")(_._1)
            log.scope("tx.head" -> txHead) { implicit log =>
              log.trace("requested") >> withWorker(appId)(
                w =>
                  workerApi
                    .sendTxAwaitResponse(w, tx, id)
                    .flatMap {
                      case Right(queryResponse) =>
                        queryResponse match {
                          case OkResponse(_, response) =>
                            log.debug(s"OK $response") >> Ok(response)
                          case RpcErrorResponse(_, r) =>
                            // TODO ERROR INCONSISTENCY: some errors are returned as 200, others as 500
                            rpcErrorToResponse(r)
                          case TimedOutResponse(txHead, tries) =>
                            log.warn(s"timed out after $tries") >>
                              // TODO: ERROR INCONSISTENCY: some errors are returned as 200, others as 500
                              RequestTimeout(
                                s"Request $txHead couldn't be processed after $tries blocks. Try later or start a new session to continue."
                              )
                          case PendingResponse(txHead) =>
                            // TODO: ERROR INCONSISTENCY: some errors are returned as 200, others as 500
                            InternalServerError(
                              s"PendingResponse is returned for tx $txHead. This shouldn't happen and means there's a serious bug, please report."
                            )
                        }
                      case Left(err) =>
                        err match {
                          // TODO: add tx.head to these responses, so it is possible to match error with transaction
                          // return an error from tendermint as is to the client
                          case TendermintResponseDeserializationError(response) =>
                            log.debug(s"error on tendermint response deserialization: $response") >> Ok(response)
                          case RpcTxAwaitError(rpcError) =>
                            log.debug(s"error on await rpc tx: $rpcError") >> rpcErrorToResponse(rpcError)
                          case TxParsingError(msg, _) =>
                            // TODO: ERROR INCONSISTENCY: some errors are returned as 200, others as 500
                            log.debug(s"error on tx parsing: $msg") >> BadRequest(msg)
                          case TxInvalidError(msg) =>
                            // TODO: ERROR INCONSISTENCY: some errors are returned as 200, others as 500
                            log.debug(s"tx is invalid: $msg") >> InternalServerError(msg)
                        }
                    }
                    .flatTap(r => log.trace(s"response: $r"))
              )
            }
          }
        }
    }
  }
}
