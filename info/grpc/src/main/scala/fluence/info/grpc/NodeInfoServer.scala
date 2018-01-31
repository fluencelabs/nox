package fluence.info.grpc

import cats.{ MonadError, ~> }
import cats.syntax.functor._
import cats.syntax.flatMap._
import fluence.info.NodeInfoRpc

import scala.concurrent.Future
import scala.language.higherKinds

class NodeInfoServer[F[_]](nodeInfo: NodeInfoRpc[F])(implicit F: MonadError[F, Throwable], run: F ~> Future)
  extends NodeInfoRpcGrpc.NodeInfoRpc {

  private val encode = NodeInfoCodec.codec[F].direct

  override def getInfo(request: NodeInfoRequest): Future[NodeInfo] =
    run(for {
      i ← nodeInfo.getInfo()
      res ← encode(i)
    } yield res)
}
