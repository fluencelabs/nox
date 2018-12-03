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

import scala.concurrent.ExecutionContext.Implicits.global
import org.http4s.server.blaze._

import scala.language.higherKinds

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

case class StateManager(config: MasterConfig, masterNode: MasterNode) {

  private val startTime = System.currentTimeMillis()

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

object StatusServerResource {

  implicit val cs: ContextShift[IO] = IO.contextShift(global)
  implicit val timer: Timer[IO] = IO.timer(global)

  def statusService(sm: StateManager): Kleisli[IO, Request[IO], Response[IO]] =
    HttpRoutes
      .of[IO] {
        case GET -> Root / "status" =>
          sm.getState.flatMap(state => Ok(state.asJson.spaces2))
      }
      .orNotFound

  def makeResource(statServerConfig: StatServerConfig, config: MasterConfig, masterNode: MasterNode
  ): Resource[IO, Server[IO]] =
    BlazeServerBuilder[IO]
      .bindHttp(statServerConfig.port, config.endpoints.ip.getHostAddress)
      .withHttpApp(statusService(StateManager(config, masterNode)))
      .resource

}
