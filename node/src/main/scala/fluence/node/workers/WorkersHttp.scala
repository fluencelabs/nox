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
import cats.syntax.flatMap._
import cats.effect.{Concurrent, Sync}
import fluence.effects.tendermint.rpc._
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
  RequestResponder,
  RequestSubscriber,
  RpcErrorResponse
}
import org.http4s.dsl.Http4sDsl
import org.http4s.{HttpRoutes, Response}

import scala.language.higherKinds

object WorkersHttp {

  def tendermintResponsetoHttp[F[_]: Monad](
    appId: Long,
    response: Option[Either[RpcError, String]]
  )(implicit log: Log[F], dsl: Http4sDsl[F]): F[Response[F]] = {
    import dsl._
    response match {
      case None ⇒
        log.debug(s"RPC Requested app $appId, but there's no such worker in the pool") *>
          NotFound("App not found on the node")

      case Some(res) ⇒
        res match {
          case Right(result) ⇒
            log.trace(s"RPC responding with OK: $result") *>
              Ok(result)

          case Left(err) ⇒
            rpcErrorToResponse(err)
        }
    }
  }

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

  /** Helper: runs a function if a worker is in a pool, unwraps EitherT into different response types, renders errors */
  def withTendermint[F[_]: Monad](
    pool: WorkersPool[F],
    appId: Long
  )(fn: TendermintRpc[F] ⇒ EitherT[F, RpcError, String])(implicit log: Log[F], dsl: Http4sDsl[F]): F[Response[F]] = {
    log.scope("app" -> appId.toString) { implicit log ⇒
      pool.withWorker(appId, _.withServices(_.tendermint)(fn(_).value)).flatMap(tendermintResponsetoHttp(appId, _))
    }
  }

  /**
   * Routes for Workers API.
   *
   * @param pool Workers pool to get workers from
   * @param dsl Http4s DSL to build routes with
   */
  def routes[F[_]: Sync: LogFactory: Concurrent](pool: WorkersPool[F], requestSubscriber: RequestSubscriber[F])(
    implicit dsl: Http4sDsl[F]
  ): HttpRoutes[F] = {
    import dsl._

    object QueryPath extends QueryParamDecoderMatcher[String]("path")
    object QueryData extends OptionalQueryParamDecoderMatcher[String]("data")
    object QueryId extends OptionalQueryParamDecoderMatcher[String]("id")

    // Routes comes there
    HttpRoutes.of {
      case GET -> Root / LongVar(appId) / "query" :? QueryPath(path) +& QueryData(data) +& QueryId(id) ⇒
        LogFactory[F].init("http" -> "query", "app" -> appId.toString) >>= { implicit log =>
          WorkersApi.query(pool, appId, data, path, id).flatMap(tendermintResponsetoHttp(appId, _))
        }

      case GET -> Root / LongVar(appId) / "status" ⇒
        LogFactory[F].init("http" -> "status", "app" -> appId.toString) >>= { implicit log =>
          WorkersApi.status(pool, appId).flatMap(tendermintResponsetoHttp(appId, _))
        }

      case GET -> Root / LongVar(appId) / "p2pPort" ⇒
        LogFactory[F].init("http" -> "p2pPort", "app" -> appId.toString) >>= { implicit log =>
          log.debug(s"Worker p2pPort") *>
            WorkersApi.p2pPort(pool, appId).flatMap {
              case Some(worker) ⇒
                log.debug(s"Worker p2pPort = ${worker.p2pPort}") *>
                  Ok(worker.p2pPort.toString)

              case None ⇒
                log.debug(s"Requested app $appId, but there's no such worker in the pool") *>
                  NotFound("App not found on the node")
            }
        }

      case req @ POST -> Root / LongVar(appId) / "tx" :? QueryId(id) ⇒
        LogFactory[F].init("http" -> "tx", "app" -> appId.toString) >>= { implicit log =>
          req.decode[String] { tx ⇒
            WorkersApi.txSync(pool, appId, tx, id).flatMap(tendermintResponsetoHttp(appId, _))
          }
        }

      case req @ POST -> Root / LongVar(appId) / "txWaitResponse" :? QueryId(id) ⇒
        LogFactory[F].init("http" -> "txWaitResponse", "app" -> appId.toString) >>= { implicit log =>
          req.decode[String] { tx ⇒
            WorkersApi.txWaitResponse(pool, requestSubscriber, appId, tx, id).flatMap {
              case Right(queryResponse) =>
                queryResponse match {
                  case OkResponse(_, responseOp) =>
                    responseOp match {
                      case Some(response) => Ok(response)
                      case None           => NotFound("App not found on the node")
                    }
                  case RpcErrorResponse(_, r) => rpcErrorToResponse(r)
                  case PendingResponse(_, _) =>
                    BadRequest("Too long time to process this query. Start a new session.")
                }
              case Left(err) =>
                err match {
                  case RpcTxSyncError(rpcError) => rpcErrorToResponse(rpcError)
                }
            }
          }
        }
    }
  }
}
