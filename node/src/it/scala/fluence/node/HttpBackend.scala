package fluence.node

import cats.Parallel
import cats.data.Kleisli
import cats.effect.{ConcurrentEffect, Resource, Timer}
import fluence.kad.http.KademliaHttp
import fluence.kad.http.dht.DhtHttp
import fluence.log.LogFactory
import fluence.node.status.{StatusAggregator, StatusHttp}
import fluence.node.workers.WorkersPorts
import org.http4s.dsl.Http4sDsl
import org.http4s.server.blaze.BlazeServerBuilder
import org.http4s.server.{Router, Server}
import org.http4s.{HttpApp, Request, Response, Status}
import shapeless.{ops, HList}

import scala.language.higherKinds

object HttpBackend {

  def make[F[_]: Parallel: Timer: ConcurrentEffect: LogFactory, RS <: HList](
    host: String,
    port: Short,
    agg: StatusAggregator[F],
    kad: KademliaHttp[F, _],
    dht: List[DhtHttp[F]] = Nil
  )(
    implicit p2p: ops.hlist.Selector[RS, WorkersPorts.P2pPort[F]]
  ): Resource[F, Server[F]] = {
    implicit val dsl: Http4sDsl[F] = Http4sDsl[F]

    val routes = Router[F](
      ("/status" -> StatusHttp.routes[F](agg)) ::
        ("/kad" -> kad.routes()) ::
        dht.map(dhtHttp ⇒ dhtHttp.prefix -> dhtHttp.routes()): _*
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

    val app: HttpApp[F] = routesOrNotFound

    BlazeServerBuilder[F]
      .bindHttp(port, host)
      .withHttpApp(app)
      .resource
  }
}
