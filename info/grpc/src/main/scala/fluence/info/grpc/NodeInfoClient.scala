package fluence.info.grpc

import cats.{ MonadError, ~> }
import cats.syntax.functor._
import cats.syntax.flatMap._
import fluence.info
import fluence.info.NodeInfoRpc
import fluence.info.grpc.NodeInfoRpcGrpc.NodeInfoRpcStub
import io.grpc.{ CallOptions, ManagedChannel }

import scala.concurrent.Future
import scala.language.higherKinds

class NodeInfoClient[F[_]](stub: NodeInfoRpcStub)(implicit run: Future ~> F, F: MonadError[F, Throwable]) extends NodeInfoRpc[F] {

  private val decode = NodeInfoCodec.codec[F].inverse

  override def getInfo(): F[info.NodeInfo] =
    for {
      rep ← run(stub.getInfo(NodeInfoRequest()))
      res ← decode(rep)
    } yield res
}

object NodeInfoClient {
  def register[F[_]]()(channel: ManagedChannel, callOptions: CallOptions)(implicit run: Future ~> F, F: MonadError[F, Throwable]): NodeInfoRpc[F] =
    new NodeInfoClient[F](new NodeInfoRpcStub(channel, callOptions))
}
