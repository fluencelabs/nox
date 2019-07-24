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

import cats.Parallel
import cats.data.Kleisli
import fluence.node.status.{StatusAggregator, StatusHttp}
import cats.effect._
import fluence.codec.PureCodec
import fluence.kad.http.KademliaHttp
import fluence.kad.protocol.Node
import fluence.log.LogFactory
import fluence.node.workers.subscription.{RequestResponderImpl, RequestSubscriber}
import fluence.node.workers.{WorkersHttp, WorkersPool}
import org.http4s.dsl.Http4sDsl
import org.http4s.{HttpApp, Request, Response, Status}
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
  def make[F[_]: Timer: ConcurrentEffect: LogFactory, G[_], C](
    host: String,
    port: Short,
    agg: StatusAggregator[F],
    pool: WorkersPool[F],
    kad: KademliaHttp[F, C],
    requestSubscriber: RequestSubscriber[F]
  )(implicit P: Parallel[F, G], writeNode: PureCodec.Func[Node[C], String]): Resource[F, Server[F]] = {
    implicit val dsl: Http4sDsl[F] = new Http4sDsl[F] {}

    val routes = Router[F](
      "/status" -> StatusHttp.routes[F, G](agg),
      "/apps" -> WorkersHttp.routes[F](pool, requestSubscriber),
      "/kad" -> kad.routes()
    )
    val routesOrNotFound = Kleisli[F, Request[F], Response[F]](
      a =>
        routes
          .run(a)
          .getOrElse(
            Response(Status.NotFound)
              .withEntity(s"Route for ${a.method} ${a.pathInfo} ${a.params.mkString("&")} not found")
        )
    )
    val app: HttpApp[F] = CORS[F, F](routesOrNotFound, corsConfig)

    BlazeServerBuilder[F]
      .bindHttp(port, host)
      .withHttpApp(app)
      .resource
  }

}
