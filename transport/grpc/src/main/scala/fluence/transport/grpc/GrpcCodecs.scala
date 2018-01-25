package fluence.transport.grpc

import cats.{ Applicative, MonadError }
import com.google.protobuf.ByteString
import fluence.codec.Codec
import fluence.kad.protocol.Key
import scodec.bits.ByteVector

import scala.language.higherKinds

object GrpcCodecs {

  implicit def byteVectorByteString[F[_] : Applicative]: Codec[F, ByteString, ByteVector] =
    Codec.pure(
      str ⇒ ByteVector(str.toByteArray),
      vec ⇒ ByteString.copyFrom(vec.toArray)
    )

  // TODO: more precise error
  implicit def keyByteString[F[_]](implicit F: MonadError[F, Throwable]): Codec[F, ByteString, Key] =
    byteVectorByteString[F] andThen Key.vectorCodec.swap
}
