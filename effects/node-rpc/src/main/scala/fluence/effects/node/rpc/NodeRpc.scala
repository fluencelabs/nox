package fluence.effects.node.rpc

import cats.Applicative
import cats.data.EitherT
import cats.effect.Resource

import scala.language.higherKinds

/**
 * RPC from state machine to node
 */
case class NodeRpc[F[_]]() {

  def uploadBlock(height: Long)(implicit F: Applicative[F]): EitherT[F, RpcError, BlockManifest] =
    EitherT.pure[F, RpcError](BlockManifest())
}

object NodeRpc {

  def make[F[_]: Applicative](hostname: String, port: Int): Resource[F, NodeRpc[F]] =
    Resource.pure(
      new NodeRpc[F]()
    )
}
