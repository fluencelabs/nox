package fluence.kad.grpc

import java.net.InetAddress
import java.time.Instant

import cats.Applicative
import fluence.codec.Codec
import fluence.kad.protocol
import fluence.kad.protocol.{ Contact, Key }
import com.google.protobuf.ByteString

import scala.language.higherKinds

object KademliaNodeCodec {
  def encode(obj: protocol.Node[Contact]): Node = Node(
    id = ByteString.copyFrom(obj.key.id),
    ByteString.copyFrom(obj.contact.ip.getAddress),
    obj.contact.port
  )

  def decode(binary: Node): protocol.Node[Contact] = protocol.Node[Contact](
    Key(binary.id.toByteArray),
    Instant.now(),
    Contact(
      InetAddress.getByAddress(binary.ip.toByteArray),
      binary.port
    )
  )

  implicit def apply[F[_] : Applicative]: Codec[F, fluence.kad.protocol.Node[Contact], Node] =
    Codec.pure(encode, decode)
}
