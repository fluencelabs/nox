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

import cats.data.EitherT
import cats.effect.{Effect, IO}
import cats.effect.syntax.effect._
import fluence.node.workers.tendermint.rpc._
import org.http4s.{HttpRoutes, Response}
import org.http4s.dsl.io._

import scala.language.higherKinds

object WorkersHttp {

  object QueryPath extends OptionalQueryParamDecoderMatcher[String]("path")
  object QueryData extends QueryParamDecoderMatcher[String]("data")

  def routes[F[_]: Effect](pool: WorkersPool[F]): HttpRoutes[IO] = {
    def withTendermint(appId: Long)(fn: TendermintRpc[F] ⇒ EitherT[F, RpcError, String]): IO[Response[IO]] =
      pool.get(appId).toIO.flatMap {
        case Some(worker) ⇒
          fn(worker.tendermint).value.toIO.flatMap {
            case Right(result) ⇒
              Ok(result)

            case Left(RpcRequestFailed(err)) ⇒
              InternalServerError(err.getMessage)

            case Left(err: RpcRequestErrored) ⇒
              InternalServerError(err.error)

            case Left(RpcBodyMalformed(err)) ⇒
              BadRequest(err.getMessage)
          }
        case None ⇒
          NotFound("App not found on the node")
      }

    HttpRoutes.of {
      case GET -> Root / LongVar(appId) / "query" :? QueryPath(path) +& QueryData(data) ⇒
        withTendermint(appId)(_.query(path, data))

      case GET -> Root / LongVar(appId) / "status" ⇒
        withTendermint(appId)(_.status)

      case req @ POST -> Root / LongVar(appId) / "tx" ⇒
        req.decode[String](tx ⇒ withTendermint(appId)(_.broadcastTxSync(tx)))
    }
  }
}
