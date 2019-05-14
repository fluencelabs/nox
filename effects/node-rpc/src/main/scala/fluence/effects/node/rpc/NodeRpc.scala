package fluence.effects.node.rpc

import cats.Applicative
import cats.data.EitherT
import cats.effect.Resource
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * RPC from state machine to node
 */
case class NodeRpc[F[_]]() {

  def uploadBlock(height: Long, vmHash: ByteVector, previousManifestReceipt: Option[Receipt])(
    implicit F: Applicative[F]
  ): EitherT[F, RpcError, Receipt] =
    EitherT.pure[F, RpcError](Receipt)
}

object NodeRpc {

  def make[F[_]: Applicative](hostname: String, port: Int): Resource[F, NodeRpc[F]] =
    Resource.pure(
      new NodeRpc[F]()
    )
}
