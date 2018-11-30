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

import cats.{Functor, Parallel}
import cats.data.{Kleisli, OptionT}
import cats.effect.{ContextShift, IO, Timer}
import fluence.node.solvers.SolversPool
import org.http4s._
import org.http4s.dsl.io._

import scala.concurrent.ExecutionContext.Implicits.global
import org.http4s.server.blaze._

import scala.language.higherKinds

case class SolverInfo(
  clusterId: String,
  codeId: String,
  uptime: Long,
  lastBlock: String,
  lastAppHash: String,
  lastBlockHeight: Int,
  state: String
)

case class MasterState(
  ip: String,
  listOfPorts: String,
  uptime: Long,
  numberOfSolvers: Integer,
  solvers: List[SolverInfo],
  config: Option[String]
)

/*class StateManager(config: MasterConfig, masterNode: MasterNode) {

  val startTime = System.currentTimeMillis()

  def getState[G[_]](implicit P: Parallel[IO, G]): MasterState = {
    val endpoints = config.endpoints
    val ports = s"${endpoints.minPort}-${endpoints.maxPort}"
    val uptime = System.currentTimeMillis() - startTime
    val solversStatus = masterNode.pool.healths.unsafeRunSync()
    val solverInfo = solversStatus.map { case (params, health) =>
      SolverInfo(
        params.clusterData.nodeInfo.node_index,
        params.clusterData.code.asString,
        health.sinceStartCommand.toMillis,
        params.clusterData.
      )
    }
    MasterState(config.endpoints.ip, ports, uptime, solversStatus.size)
  }
}*/

object StatusServerResource {

  implicit val cs: ContextShift[IO] = IO.contextShift(global)
  implicit val timer: Timer[IO] = IO.timer(global)

  def orNotFound[F[_]: Functor, A](self: Kleisli[OptionT[F, ?], A, Response[F]]): Kleisli[F, A, Response[F]] =
    Kleisli(a => self.run(a).getOrElse(Response.notFound))

  def statusService(pool: SolversPool[IO]) = orNotFound(
    HttpRoutes
      .of[IO] {
        case GET -> Root / "status" =>
          for {
            health <- pool.healths
            response <- Ok(health.mkString("\n"))
          } yield {
            response
          }
      }
  )

  def makeResource(pool: SolversPool[IO]) =
    BlazeServerBuilder[IO]
      .bindHttp(5678, "localhost")
      .withHttpApp(statusService(pool))
      .resource

}
