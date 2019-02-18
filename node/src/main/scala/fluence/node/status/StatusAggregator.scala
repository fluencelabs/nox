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

import cats.Traverse
import cats.data.Kleisli
import cats.effect._
import cats.instances.list._
import cats.syntax.applicativeError._
import fluence.node.MasterNode
import fluence.node.config.MasterConfig
import io.circe.syntax._
import org.http4s._
import org.http4s.dsl.io._
import org.http4s.implicits._
import org.http4s.server.Server
import org.http4s.server.blaze._
import org.http4s.server.middleware.{CORS, CORSConfig}
import slogging.LazyLogging

import scala.concurrent.duration._
import scala.language.higherKinds

/**
 * The manager that able to get information about master node and all workers.
 *
 * @param config config file about a master node
 * @param masterNode initialized master node
 */
case class StatusAggregator(config: MasterConfig, masterNode: MasterNode[IO], startTimeMillis: Long)(
  implicit timer: Timer[IO]
) {

  /**
   * Gets all state information about master node and workers.
   * @return gathered information
   */
  val getStatus: IO[MasterStatus] = for {
    currentTime ← timer.clock.monotonic(MILLISECONDS)
    workers ← masterNode.pool.getAll
    workerInfos ← Traverse[List].traverse(workers)(_.status)
    ethState ← masterNode.nodeEth.expectedState
  } yield
    MasterStatus(
      config.endpoints.ip.getHostName,
      currentTime - startTimeMillis,
      masterNode.nodeConfig,
      workerInfos.size,
      workerInfos,
      config,
      ethState
    )
}

object StatusAggregator extends LazyLogging {

  val corsConfig = CORSConfig(
    anyOrigin = true,
    anyMethod = true,
    allowedMethods = Some(Set("GET", "POST")),
    allowCredentials = true,
    maxAge = 1.day.toSeconds
  )

  private def statusService(
    sm: StatusAggregator
  )(implicit cs: ContextShift[IO]): Kleisli[IO, Request[IO], Response[IO]] =
    CORS(
      HttpRoutes
        .of[IO] {
          case GET -> Root / "status" =>
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
        .orNotFound,
      corsConfig
    )

  /**
   * Makes the server that gives gathered information about a master node and workers.
   *
   * @param masterConfig parameters about a master node
   * @param masterNode initialized master node
   */
  def makeHttpResource(
    masterConfig: MasterConfig,
    masterNode: MasterNode[IO]
  )(
    implicit cs: ContextShift[IO],
    timer: Timer[IO]
  ): Resource[IO, Server[IO]] =
    for {
      startTimeMillis ← Resource.liftF(timer.clock.realTime(MILLISECONDS))
      _ = logger.debug("Start time millis: " + startTimeMillis)
      server ← BlazeServerBuilder[IO]
        .bindHttp(masterConfig.statusServer.port, "0.0.0.0")
        .withHttpApp(statusService(StatusAggregator(masterConfig, masterNode, startTimeMillis)))
        .resource
    } yield server
}
