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
import cats.effect.{ContextShift, IO, Resource, Timer}
import fluence.node.config.{MasterConfig, StatusServerConfig}
import fluence.node.solvers.SolverInfo
import org.http4s._
import org.http4s.implicits._
import scala.concurrent.duration._
import org.http4s.dsl.io._
import io.circe.syntax._
import io.circe.generic.semiauto._
import io.circe.Encoder
import org.http4s.server.Server

import scala.concurrent.duration.MILLISECONDS
import org.http4s.server.blaze._
import org.http4s.server.middleware.{CORS, CORSConfig}

import scala.language.higherKinds

/**
 * Master node state.
 *
 * @param ip master node ip address
 * @param listOfPorts all available ports to use by code developers
 * @param uptime working time of master node
 * @param numberOfSolvers number of registered solvers
 * @param solvers info about solvers
 * @param config config file
 */
case class MasterStatus(
  ip: String,
  listOfPorts: String,
  uptime: Long,
  numberOfSolvers: Int,
  solvers: List[SolverInfo],
  config: MasterConfig
)

object MasterStatus {
  implicit val encodeMasterState: Encoder[MasterStatus] = deriveEncoder
}

/**
 * The manager that able to get information about master node and all solvers.
 *
 * @param config config file about a master node
 * @param masterNode initialized master node
 */
case class StatusAggregator(config: MasterConfig, masterNode: MasterNode, startTimeMillis: Long)(
  implicit timer: Timer[IO]
) {

  /**
   * Gets all state information about master node and solvers.
   * @return gathered information
   */
  def getStatus[G[_]](implicit P: Parallel[IO, G]): IO[MasterStatus] = {
    val endpoints = config.endpoints
    val ports = s"${endpoints.minPort}:${endpoints.maxPort}"
    for {
      currentTime <- timer.clock.monotonic(MILLISECONDS)
      solversStatus <- masterNode.pool.healths
      solverInfos = solversStatus.values.toList
    } yield
      MasterStatus(
        config.endpoints.ip.getHostName,
        ports,
        currentTime - startTimeMillis,
        solversStatus.size,
        solverInfos,
        config
      )
  }
}

object StatusAggregator {

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
            sm.getStatus.flatMap(state => Ok(state.asJson.spaces2))
        }
        .orNotFound,
      corsConfig
    )

  /**
   * Makes the server that gives gathered information about a master node and solvers.
   *
   * @param statServerConfig server's parameters
   * @param masterConfig parameters about a master node
   * @param masterNode initialized master node
   */
  def makeHttpResource(
    statServerConfig: StatusServerConfig,
    masterConfig: MasterConfig,
    masterNode: MasterNode,
    startTimeMillis: Long
  )(
    implicit cs: ContextShift[IO],
    timer: Timer[IO]
  ): Resource[IO, Server[IO]] =
    BlazeServerBuilder[IO]
      .bindHttp(statServerConfig.port, "0.0.0.0")
      .withHttpApp(statusService(StatusAggregator(masterConfig, masterNode, startTimeMillis)))
      .resource

}
