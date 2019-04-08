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
import org.http4s.HttpRoutes
import cats.syntax.applicativeError._
import io.circe.syntax._
import org.http4s.dsl._
import org.http4s.dsl.impl.OptionalQueryParamDecoderMatcher
import slogging.LazyLogging

import scala.concurrent.duration._
import scala.language.higherKinds

object StatusHttp extends LazyLogging {

  object Timeout extends OptionalQueryParamDecoderMatcher[Int]("timeout")

  /**
   * Master status' routes.
   *
   * @param sm Status aggregator
   * @param dsl Http4s DSL to build routes with
   */
  def routes[F[_]: Sync, G[_]](
    sm: StatusAggregator[F]
  )(implicit dsl: Http4sDsl[F], P: Parallel[F, G]): HttpRoutes[F] = {
    import dsl._
    HttpRoutes
      .of[F] {
        case GET -> Root :? Timeout(t) =>
          (for {
            status <- sm.getStatus(t.getOrElse(5).seconds)
            maybeJson <- Sync[F].delay(status.asJson.spaces2).attempt
          } yield (status, maybeJson)).flatMap {
            case (status, Left(e)) ⇒
              logger.error(s"Status cannot be serialized to JSON. Status: $status", e)
              e.printStackTrace()
              InternalServerError("JSON generation errored, please try again")

            case (_, Right(json)) ⇒
              Ok(json)
          }
      }
  }
}
