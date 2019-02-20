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

package fluence.node

import fluence.node.status.{StatusAggregator, StatusHttp}
import cats.effect._
import fluence.node.workers.{WorkersHttp, WorkersPool}
import org.http4s.dsl.Http4sDsl
import org.http4s.{HttpApp, HttpRoutes}
import org.http4s.implicits._
import org.http4s.server.{Router, Server}
import org.http4s.server.blaze._
import org.http4s.server.middleware.{CORS, CORSConfig}

import scala.concurrent.duration._
import scala.language.higherKinds

object MasterHttp {

  private val corsConfig = CORSConfig(
    anyOrigin = true,
    anyMethod = true,
    allowedMethods = Some(Set("GET", "POST")),
    allowCredentials = true,
    maxAge = 1.day.toSeconds
  )

  /**
   * Makes a HTTP server with all the expected routes.
   *
   * @param port Port to bind to
   * @param agg Status Aggregator
   * @param pool Workers Pool
   */
  def make[F[_]: Timer: ConcurrentEffect](
    host: String,
    port: Short,
    agg: StatusAggregator[F],
    pool: WorkersPool[F]
  ): Resource[F, Server[F]] = {
    implicit val dsl: Http4sDsl[F] = new Http4sDsl[F] {}

    val routes: HttpRoutes[F] = Router[F](
      "/status" -> StatusHttp.routes[F](agg),
      "/apps" -> WorkersHttp.routes[F](pool)
    )

    val app: HttpApp[F] = CORS[F, F](routes.orNotFound, corsConfig)

    BlazeServerBuilder[F]
      .bindHttp(port, host)
      .withHttpApp(app)
      .resource
  }

}
