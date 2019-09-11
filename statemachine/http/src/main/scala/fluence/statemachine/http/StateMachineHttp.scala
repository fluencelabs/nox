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

package fluence.statemachine.http

import cats.Monad
import cats.effect.Sync
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.log.{Log, LogFactory}
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.command.{PeersControl, ReceiptBus}
import org.http4s.dsl.Http4sDsl
import org.http4s.{HttpRoutes, Request}
import io.circe.syntax._
import shapeless._

import scala.language.higherKinds

// TODO implement txProcessor routes, providing end-user interface for it
object StateMachineHttp {
  private[http] def logReq[F[_]: LogFactory: Monad](req: Request[F]): F[Log[F]] =
    LogFactory[F]
      .init("method" -> req.method.toString(), "path" -> req.pathInfo)
      .flatTap(_.info(s"request"))
      .widen[Log[F]]

  /**
   * Routes for a pure readonly [[StateMachine]]
   */
  def readRoutes[F[_]: Http4sDsl: LogFactory: Sync](
    stateMachine: StateMachine[F]
  )(implicit dsl: Http4sDsl[F]): HttpRoutes[F] = {
    import dsl._

    object PathQ extends QueryParamDecoderMatcher[String]("path")

    HttpRoutes.of[F] {
      case req @ GET -> Root / "status" ⇒
        for {
          implicit0(log: Log[F]) ← logReq[F](req)
          status ← stateMachine.status().value
          res ← status.fold(
            err ⇒ InternalServerError(err.getMessage),
            st ⇒ Ok(st.asJson.noSpaces)
          )
        } yield res

      case req @ GET -> Root / "query" :? PathQ(path) ⇒
        for {
          implicit0(log: Log[F]) ← logReq[F](req)
          queryResp ← stateMachine.query(path).value
          res ← queryResp.fold(
            err ⇒ InternalServerError(err.getMessage),
            resp ⇒ Ok(resp.toResponseString())
          )
        } yield res
    }
  }

  /**
   * Routes for the [[StateMachine]]'s command side
   */
  def commandRoutes[F[_]: Http4sDsl: LogFactory: Sync](
    receiptBus: ReceiptBus[F],
    peers: PeersControl[F]
  ): Seq[(String, HttpRoutes[F])] =
    Seq(
      "/receipt-bus" -> ReceiptBusHttp.routes[F](receiptBus),
      "/peers" -> PeersControlHttp.routes[F](peers)
    )

  /**
   * List of all available [[StateMachine]]'s routes.
   *
   * @param machine [[StateMachine]] with at least [[ReceiptBus]] and [[PeersControl]] on the command side
   * @return List of routes to be passed into server's Router
   */
  def routes[F[_]: Http4sDsl: LogFactory: Sync, C <: HList](
    machine: StateMachine.Aux[F, C]
  )(
    implicit rb: ops.hlist.Selector[C, ReceiptBus[F]],
    pc: ops.hlist.Selector[C, PeersControl[F]]
  ): Seq[(String, HttpRoutes[F])] =
    commandRoutes[F](
      machine.command[ReceiptBus[F]],
      machine.command[PeersControl[F]]
    ) :+ ("/" -> readRoutes[F](machine))
}
