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

package fluence.node.status

import cats.Parallel
import cats.effect.Sync
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.apply._
import org.http4s.HttpRoutes
import cats.syntax.applicativeError._
import fluence.log.{Log, LogFactory}
import io.circe.syntax._
import org.http4s.dsl._
import org.http4s.dsl.impl.OptionalQueryParamDecoderMatcher

import scala.concurrent.duration._
import scala.language.higherKinds

object StatusHttp {

  // Timeout in seconds
  object Timeout extends OptionalQueryParamDecoderMatcher[Int]("timeout")

  /**
   * Master status' routes.
   *
   * @param sm Status aggregator
   * @param dsl Http4s DSL to build routes with
   */
  def routes[F[_]: Sync: LogFactory, G[_]](
    sm: StatusAggregator[F],
    defaultTimeout: FiniteDuration = 5.seconds
  )(implicit dsl: Http4sDsl[F], P: Parallel[F, G]): HttpRoutes[F] = {
    import dsl._

    val maxTimeout = defaultTimeout * 20

    HttpRoutes
      .of[F] {
        case GET -> Root :? Timeout(t) =>
          (for {
            implicit0(log: Log[F]) ← LogFactory[F].init("http", "status")
            status <- sm.getStatus(t.map(_.seconds).filter(_ < maxTimeout).getOrElse(defaultTimeout))
            maybeJson <- Sync[F].delay(status.asJson.spaces2).attempt
          } yield (log, status, maybeJson)).flatMap {
            case (log, status, Left(e)) ⇒
              log.error(s"Status cannot be serialized to JSON. Status: $status", e) *>
                InternalServerError("JSON generation errored, please try again")

            case (_, _, Right(json)) ⇒
              Ok(json)
          }

        case GET -> Root / "eth" ⇒
          import MasterStatus.encodeNodeEthState

          (for {
            implicit0(log: Log[F]) ← LogFactory[F].init("http", "status/eth")
            status ← sm.expectedEthState
            maybeJson ← Sync[F].delay(status.asJson.spaces2).attempt
          } yield (log, status, maybeJson)).flatMap {
            case (log, status, Left(e)) ⇒
              log.error(s"Status cannot be serialized to JSON. Status: $status", e) *>
                InternalServerError("JSON generation errored, please try again")

            case (_, _, Right(json)) ⇒
              Ok(json)
          }
      }
  }
}
