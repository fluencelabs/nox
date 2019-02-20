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

import cats.effect.Sync
import cats.syntax.functor._
import cats.syntax.flatMap._
import org.http4s.HttpRoutes
import cats.syntax.applicativeError._
import io.circe.syntax._
import org.http4s.dsl._
import slogging.LazyLogging

import scala.language.higherKinds
import scala.util.control.NonFatal

object StatusHttp extends LazyLogging {

  /**
   * Master status' routes.
   *
   * @param sm Status aggregator
   * @param dsl Http4s DSL to build routes with
   */
  def routes[F[_]: Sync](sm: StatusAggregator[F])(implicit dsl: Http4sDsl[F]): HttpRoutes[F] = {
    import dsl._
    HttpRoutes
      .of[F] {
        case GET -> Root =>
          for {
            status <- sm.getStatus
            json <- Sync[F].delay(status.asJson.spaces2).handleError {
              case NonFatal(e) =>
                e.printStackTrace()
                logger.error(s"Status cannot be serialized to JSON. Status: $status", e)
                "\"JSON generation errored, please try again\""
            }
            response <- Ok(json)
            _ <- Sync[F].delay(logger.trace("MasterStatus responded successfully"))
          } yield response
      }
  }
}
