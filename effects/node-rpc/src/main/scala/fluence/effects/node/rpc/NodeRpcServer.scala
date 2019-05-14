package fluence.effects.node.rpc

import cats.Functor
import cats.data.Kleisli
import cats.effect.{ConcurrentEffect, Resource, Sync, Timer}
import cats.syntax.applicative._
import cats.syntax.flatMap._
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.TendermintRpc
import org.http4s.circe._
import org.http4s.dsl.Http4sDsl
import org.http4s.implicits._
import org.http4s.server.Server
import org.http4s.server.blaze.BlazeServerBuilder
import org.http4s.{EntityDecoder, EntityEncoder, HttpRoutes, Request, Response}

import scala.language.higherKinds

case class NodeRpcServer[F[_]](http: Server[F]) {}

object NodeRpcServer extends slogging.LazyLogging {
  private def nodeRpcService[F[_]: Functor: Sync](
    blockHistory: BlockHistory[F],
    tendermintRpc: TendermintRpc[F]
  )(implicit dsl: Http4sDsl[F]): Kleisli[F, Request[F], Response[F]] = {
    import dsl._

    implicit val decoder: EntityDecoder[F, UploadBlock] = jsonOf[F, UploadBlock]
    implicit val encoder: EntityEncoder[F, Receipt] = jsonEncoderOf[F, Receipt]

    val route: PartialFunction[Request[F], F[Response[F]]] = {
      case req @ POST -> Root / "uploadBlock" =>
        req.as[UploadBlock].flatMap {
          case UploadBlock(height, hash, receipt) =>
            tendermintRpc.block(height).subflatMap(Block(_)).value.flatMap {
              case Left(e) =>
                Response[F](InternalServerError).withEntity(s"Error while retrieving block $height: $e").pure[F]
              case Right(block) =>
                blockHistory.upload(block, hash, receipt).flatMap(Ok(_))
            }
        }

      case _ => Sync[F].pure(Response.notFound)
    }

    val log: PartialFunction[Request[F], Request[F]] = {
      case req =>
        logger.info(s"RPC REQ: [${req.pathInfo}] $req")
        req
    }

    HttpRoutes
      .of[F] { log.andThen(route) }
      .orNotFound
  }

  def make[F[_]: ConcurrentEffect: Timer](host: String, port: Int): Resource[F, NodeRpcServer[F]] = {
    implicit val dsl: Http4sDsl[F] = new Http4sDsl[F] {}

    //TODO: it will require IPFS
    val blockHistory = BlockHistory[F]()
    for {
      // TODO: Maybe it was a bad idea to use TendermintRPC here?
      // TODO (cont): because I should identify which app's statemachine request is from, so I can query correct tendermint...
      rpc <- TendermintRpc.make[F]()
      server ← BlazeServerBuilder[F]
        .bindHttp(port, host)
        .withHttpApp(nodeRpcService(blockHistory))
        .resource
    } yield NodeRpcServer(server)
  }
}
