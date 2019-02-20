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

import cats.effect.{Effect, IO}
import org.http4s.HttpRoutes
import cats.syntax.applicativeError._
import io.circe.syntax._
import org.http4s.dsl.io._
import slogging.LazyLogging

import scala.language.higherKinds

object StatusHttp extends LazyLogging {

  def routes[F[_]: Effect](sm: StatusAggregator): HttpRoutes[IO] =
    HttpRoutes
      .of[IO] {
        case GET -> Root =>
          val response = for {
            status <- sm.getStatus
            json <- IO(status.asJson.spaces2).onError {
              case e =>
                IO(e.printStackTrace())
                  .map(_ => logger.error(s"Status cannot be serialized to JSON. Status: $status", e))
            }
            response <- Ok(json)
            _ <- IO(logger.trace("MasterStatus responded successfully"))
          } yield response

          response.handleErrorWith { e =>
            val errorMessage = s"Cannot produce MasterStatus response: $e"
            logger.warn(errorMessage)
            e.printStackTrace()
            InternalServerError(errorMessage)
          }
      }
}
