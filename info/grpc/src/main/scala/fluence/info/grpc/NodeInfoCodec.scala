package fluence.info.grpc

import cats.MonadError
import fluence.codec.Codec

import scala.language.higherKinds

object NodeInfoCodec {

  implicit def codec[F[_]](implicit F: MonadError[F, Throwable]): Codec[F, fluence.info.NodeInfo, NodeInfo] =
    Codec(
      encode = i ⇒ F.catchNonFatal(NodeInfo(i.build)),
      decode = g ⇒ F.catchNonFatal(fluence.info.NodeInfo(g.build))
    )

}
