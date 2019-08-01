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
import fluence.node.workers.WorkersApi.{
  AppNotFoundError,
  RpcTxAwaitError,
  TendermintResponseError,
  TxInvalidError,
  TxParsingError
}
import fluence.node.workers.subscription.{OkResponse, ResponseSubscriber, RpcErrorResponse, TimedOutResponse}
import org.http4s.dsl.Http4sDsl
import org.http4s.{HttpRoutes, Response}

import scala.language.higherKinds

object WorkersHttp {

  /**
   * Encodes a tendermint response to HTTP format.
   *
   */
  def tendermintResponseToHttp[F[_]: Monad](
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
  def routes[F[_]: Sync: LogFactory: Concurrent](pool: WorkersPool[F])(
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
          WorkersApi.query(pool, appId, data, path, id).flatMap(tendermintResponseToHttp(appId, _))
        }

      case GET -> Root / LongVar(appId) / "status" ⇒
        LogFactory[F].init("http" -> "status", "app" -> appId.toString) >>= { implicit log =>
          WorkersApi.status(pool, appId).flatMap(tendermintResponseToHttp(appId, _))
        }

      case GET -> Root / LongVar(appId) / "p2pPort" ⇒
        LogFactory[F].init("http" -> "p2pPort", "app" -> appId.toString) >>= { implicit log =>
          log.debug(s"Worker p2pPort") *>
            WorkersApi.p2pPort(pool, appId).flatMap {
              case Some(p2pPort) ⇒
                log.debug(s"Worker p2pPort = $p2pPort") *>
                  Ok(p2pPort.toString)

              case None ⇒
                log.debug(s"Requested app $appId, but there's no such worker in the pool") *>
                  NotFound(s"App $appId not found on the node")
            }
        }

      case GET -> Root / LongVar(appId) / "lastManifest" ⇒
        LogFactory[F].init("http" -> "lastManifest", "app" -> appId.toString) >>= { implicit log =>
          WorkersApi.lastManifest(pool, appId).flatMap {
            case Some(worker) ⇒
              worker match {
                case Some(m) ⇒ Ok(m.jsonString)
                case None ⇒
                  log.debug("There's no available manifest yet") *>
                    NoContent()
              }

            case None ⇒
              // TODO try to get last manifest from local Kademlia storage
              log.debug(s"Requested app $appId, but there's no such worker in the pool") *>
                NotFound("App not found on the node")
          }
        }

      case req @ POST -> Root / LongVar(appId) / "tx" :? QueryId(id) ⇒
        LogFactory[F].init("http" -> "tx", "app" -> appId.toString) >>= { implicit log =>
          req.decode[String] { tx ⇒
            WorkersApi.broadcastTx(pool, appId, tx, id).flatMap(tendermintResponseToHttp(appId, _))
          }
        }

      case req @ POST -> Root / LongVar(appId) / "txWaitResponse" :? QueryId(id) ⇒
        LogFactory[F].init("http" -> "txAwaitResponse", "app" -> appId.toString) >>= { implicit log =>
          req.decode[String] { tx ⇒
            WorkersApi.txAwaitResponse(pool, appId, tx, id).flatMap {
              case Right(queryResponse) =>
                queryResponse match {
                  case OkResponse(_, response) =>
                    Ok(response)
                  case RpcErrorResponse(_, r) => rpcErrorToResponse(r)
                  case TimedOutResponse(id, tries) =>
                    RequestTimeout(
                      s"Request $id couldn't be processed after $tries blocks. Try later or start a new session to continue."
                    )
                }
              case Left(err) =>
                err match {
                  // return an error from tendermint as is to the client
                  case TendermintResponseError(response) => Ok(response)
                  case RpcTxAwaitError(rpcError)         => rpcErrorToResponse(rpcError)
                  case TxParsingError(msg, tx)           => BadRequest(msg)
                  case AppNotFoundError(msg)             => BadRequest(msg)
                  case TxInvalidError(msg)               => InternalServerError(msg)
                }
            }
          }
        }
    }
  }
}
