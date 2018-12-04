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
import fluence.node.config.{MasterConfig, StatServerConfig}
import fluence.node.solvers.SolverHealth
import org.http4s._
import org.http4s.implicits._
import org.http4s.dsl.io._
import io.circe.syntax._
import io.circe.generic.semiauto._
import io.circe.Encoder
import org.http4s.server.Server

import org.http4s.server.blaze._

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
case class MasterState(
  ip: String,
  listOfPorts: String,
  uptime: Long,
  numberOfSolvers: Int,
  solvers: List[SolverHealth],
  config: MasterConfig
)

object MasterState {
  implicit val encodeMasterState: Encoder[MasterState] = deriveEncoder
}

/**
 * The manager that able to get information about master node and all solvers.
 *
 * @param config config file about a master node
 * @param masterNode initialized master node
 */
case class StateManager(config: MasterConfig, masterNode: MasterNode) {

  private val startTime = System.currentTimeMillis()

  /**
   * Gets all state information about master node and solvers.
   * @return gathered information
   */
  def getState[G[_]](implicit P: Parallel[IO, G]): IO[MasterState] = {
    val endpoints = config.endpoints
    val ports = s"${endpoints.minPort}:${endpoints.maxPort}"
    val uptime = System.currentTimeMillis() - startTime
    for {
      solversStatus <- masterNode.pool.healths
      solverInfos = solversStatus.values.toList
    } yield MasterState(config.endpoints.ip.getHostName, ports, uptime, solversStatus.size, solverInfos, config)
  }
}

object StateManager {

  private def statusService(sm: StateManager)(implicit cs: ContextShift[IO]): Kleisli[IO, Request[IO], Response[IO]] =
    HttpRoutes
      .of[IO] {
        case GET -> Root / "status" =>
          sm.getState.flatMap(state => Ok(state.asJson.spaces2))
      }
      .orNotFound

  /**
   * Makes the server that gives gathered information about a master node and solvers.
   *
   * @param statServerConfig server's parameters
   * @param masterConfig parameters about a master node
   * @param masterNode initialized master node
   * @return
   */
  def makeResource(statServerConfig: StatServerConfig, masterConfig: MasterConfig, masterNode: MasterNode)(
    implicit cs: ContextShift[IO],
    timer: Timer[IO]
  ): Resource[IO, Server[IO]] =
    BlazeServerBuilder[IO]
      .bindHttp(statServerConfig.port, masterConfig.endpoints.ip.getHostAddress)
      .withHttpApp(statusService(StateManager(masterConfig, masterNode)))
      .resource

}
