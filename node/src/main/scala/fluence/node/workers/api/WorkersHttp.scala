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

package fluence.node.workers.api

import cats.effect.{Concurrent, Sync, Timer}
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Monad, Parallel}
import fluence.bp.tx.Tx
import fluence.effects.tendermint.rpc.http._
import fluence.log.{Log, LogFactory}
import fluence.node.MasterPool
import fluence.node.workers.WorkersPorts
import fluence.worker.WorkerStage
import fluence.worker.responder.WorkerResponder
import fluence.worker.responder.resp._
import fs2.concurrent.Queue
import io.circe.syntax._
import org.http4s.circe._
import org.http4s.dsl.Http4sDsl
import org.http4s.server.websocket.WebSocketBuilder
import org.http4s.websocket.WebSocketFrame
import org.http4s.websocket.WebSocketFrame.{Close, Text}
import org.http4s.{EntityEncoder, HttpRoutes, Response}

import scala.concurrent.duration._
import scala.language.higherKinds

object WorkersHttp {

  /**
   * Encodes a tendermint response to HTTP format.
   *
   */
  private def tendermintResponseToHttp[F[_]: Monad, T: EntityEncoder[F, ?]](
    appId: Long,
    response: Either[RpcError, T]
  )(implicit log: Log[F], dsl: Http4sDsl[F]): F[Response[F]] = {
    import dsl._
    response match {
      case Right(result) ⇒
        log.trace(s"RPC responding with OK: $result") *>
          Ok.apply(result)

      case Left(err) ⇒
        rpcErrorToResponse(err)
    }
  }

  /**
   * Encodes errors to HTTP format.
   *
   */
  private def rpcErrorToResponse[F[_]: Monad](
    error: RpcError
  )(implicit log: Log[F], dsl: Http4sDsl[F]): F[Response[F]] = {
    import dsl._
    error match {
      case RpcRequestFailed(err) ⇒
        log.warn(s"RPC request failed", err) *>
          InternalServerError(err.getMessage)

      case err: RpcHttpError ⇒
        log.warn(s"RPC request errored", err) *>
          InternalServerError(err.error)

      case err: RpcCallError =>
        log.warn(s"RPC call resulted in error", err) *>
          // TODO: is it OK to return BadRequest here?
          BadRequest(err.getMessage)

      case err: RpcBodyMalformed ⇒
        log.warn(s"RPC body malformed: $err", err) *>
          BadRequest(err.getMessage)

      case err: RpcBlockParsingFailed =>
        log.warn(s"RPC $err", err) *>
          InternalServerError(err.getMessage)
    }
  }

  /**
   * Routes for Workers API.
   *
   * @param pool Workers pool to get workers from
   * @param dsl Http4s DSL to build routes with
   */
  def routes[F[_]: Sync: LogFactory: Concurrent: Timer: Parallel](pool: MasterPool.Type[F])(
    implicit dsl: Http4sDsl[F]
  ): HttpRoutes[F] = {
    import dsl._

    object QueryPath extends QueryParamDecoderMatcher[String]("path")
    object QueryData extends OptionalQueryParamDecoderMatcher[String]("data")
    object QueryId extends OptionalQueryParamDecoderMatcher[String]("id")
    object QueryWait extends OptionalQueryParamDecoderMatcher[Int]("wait")

    def api(appId: Long) =
      for {
        worker ← pool.getWorker(appId)
        producer = worker.producer
        machine = worker.machine
        // TODO: querying worker map several times, could be optimized
        responder ← pool.getCompanion[WorkerResponder[F]](appId)
        p2pPort ← pool
          .getResources(appId)
          .map(_.select[WorkersPorts.P2pPort[F]])
          .toRight[WorkerStage](WorkerStage.NotInitialized) // TODO: How to go from OptionT to EitherT nicer?
      } yield WorkerApi(producer, responder, machine, p2pPort)

    def wrongStageMsg(appId: Long, stage: WorkerStage) = s"Worker for $appId can't serve RPC: it is in stage $stage"

//    def withWorker(appId: Long)(fn: Worker[F, MasterPool.Companions[F] ⇒ F[Response[F]])

    def withApi(appId: Long)(fn: WorkerApi[F] => F[Response[F]])(implicit log: Log[F]): F[Response[F]] =
      api(appId).value.flatMap {
        case Left(stage) =>
          log.debug(wrongStageMsg(appId, stage)) *>
            NotFound(wrongStageMsg(appId, stage))
        case Right(api) =>
          fn(api)
      }

    def status(appId: Long, timeout: FiniteDuration)(implicit log: Log[F]): F[Response[F]] =
      pool.getWorker(appId).value.flatMap {
        case Left(stage) =>
          log.debug(wrongStageMsg(appId, stage)) *>
            NotFound(wrongStageMsg(appId, stage))
        case Right(worker) ⇒ worker.status(timeout).flatMap(s ⇒ Ok(s.asJson.spaces2))
      }

    // Routes comes there
    HttpRoutes.of {
      case GET -> Root / LongVar(appId) / "ws" =>
        LogFactory[F].init("http" -> "websocket", "app" -> appId.toString) >>= { implicit log =>
          withApi(appId) { w ⇒
            w.websocket().flatMap { ws =>
              val processMessages: fs2.Pipe[F, WebSocketFrame, WebSocketFrame] =
                _.evalMap {
                  case Text(msg, _) =>
                    ws.processRequest(msg).map(Text(_))
                  case Close(data) =>
                    ws.closeWebsocket().map(_ => Text("Closing websocket"))
                  case m => log.error(s"Unsupported message: $m") as Text("Unsupported")
                }

              // TODO add maxSize to a config
              Queue
                .bounded[F, WebSocketFrame](32)
                .flatMap { q =>
                  val d = q.dequeue.through(processMessages).merge(ws.subscriptionEventStream.map(Text(_)))
                  val e = q.enqueue
                  WebSocketBuilder[F].build(d, e)
                }
            }
          }
        }

      case GET -> Root / LongVar(appId) / "query" :? QueryPath(path) +& QueryData(data) +& QueryId(id) ⇒
        LogFactory[F].init("http" -> "query", "app" -> appId.toString) >>= { implicit log =>
          withApi(appId)(_.query(data, path, id).flatMap(tendermintResponseToHttp(appId, _)))
        }

      case GET -> Root / LongVar(appId) / "status" :? QueryWait(wait) ⇒
        LogFactory[F].init("http" -> "status", "app" -> appId.toString) >>= { implicit log =>
          // Fetches the worker's status, waiting no more than 10 seconds (if ?wait=$SECONDS is provided), or 1 second otherwise
          status(appId, wait.filter(_ < 10).fold(1.second)(_.seconds))
        }

      case GET -> Root / LongVar(appId) / "p2pPort" ⇒
        LogFactory[F].init("http" -> "p2pPort", "app" -> appId.toString) >>= { implicit log =>
          log.trace(s"Worker p2pPort") *>
            withApi(appId)(_.p2pPort().map(_.toString).flatMap(Ok(_)))
        }

      case req @ POST -> Root / LongVar(appId) / "tx" ⇒
        LogFactory[F].init("http" -> "tx", "app" -> appId.toString) >>= { implicit log =>
          req.decode[Array[Byte]] { tx ⇒
            withApi(appId)(
              _.sendTx(tx)
                .flatTap(_ => log.debug(s"tx.head: ${tx.takeWhile(_ != '\n')}"))
                .flatMap(r => tendermintResponseToHttp(appId, r.map(_.asJson)))
            )
          }
        }

      case req @ POST -> Root / LongVar(appId) / "txWaitResponse" ⇒
        LogFactory[F].init("http" -> "txAwaitResponse", "app" -> appId.toString) >>= { log =>
          req.decode[Array[Byte]] { tx ⇒
            val txHead = Tx.splitTx(tx).fold("not parsed")(_._1)
            log.scope("tx.head" -> txHead) { implicit log =>
              log.trace("requested") >> withApi(appId)(
                _.sendTxAwaitResponse(tx).flatMap {
                  case Right(queryResponse) =>
                    queryResponse match {
                      case OkResponse(_, response) =>
                        log.debug(s"OK $response") >> Ok(response)
                      case RpcErrorResponse(id, r) =>
                        // TODO ERROR INCONSISTENCY: some errors are returned as 200, others as 500
                        log.warn(s"Error on $id while querying response: ${r.getMessage}") >>
                          InternalServerError(s"Error on $id while querying response: ${r.getMessage}")
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
                      case RpcTxAwaitError(rpcError) =>
                        log.debug(s"error on await rpc tx: $rpcError") >> rpcErrorToResponse(RpcRequestFailed(rpcError))
                      case TxParsingError(msg, _) =>
                        // TODO: ERROR INCONSISTENCY: some errors are returned as 200, others as 500
                        log.debug(s"error on tx parsing: $msg") >> BadRequest(msg)
                      case TxInvalidError(msg) =>
                        // TODO: ERROR INCONSISTENCY: some errors are returned as 200, others as 500
                        log.debug(s"tx is invalid: $msg") >> InternalServerError(msg)
                    }
                }.flatTap(r => log.trace(s"response: $r"))
              )
            }
          }
        }
    }
  }
}
