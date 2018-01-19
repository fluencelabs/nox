package fluence.dataset.grpc

import java.nio.ByteBuffer

import cats.{ MonadError, Traverse }
import cats.syntax.functor._
import cats.syntax.flatMap._
import com.google.protobuf.ByteString
import fluence.codec.Codec
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signature
import fluence.kad.protocol.Key
import cats.instances.list._

import scala.language.higherKinds

object BasicContractCodec {

  implicit def codec[F[_]](implicit F: MonadError[F, Throwable]): Codec[F, fluence.dataset.BasicContract, BasicContract] =
    Codec(
      bc ⇒ F catchNonFatal BasicContract(
        id = ByteString.copyFrom(bc.id.origin),
        publicKey = ByteString.copyFrom(bc.offerSeal.publicKey.value),

        participantsRequired = bc.offer.participantsRequired,

        participants = bc.participants.map {
          case (p, s) ⇒ Participant(id = ByteString.copyFrom(p.origin), publicKey = ByteString.copyFrom(s.publicKey.value), signature = ByteString.copyFrom(s.sign))
        }.toSeq,

        participantsSeal = bc.participantsSeal.map(_.sign).map(ByteString.copyFrom).getOrElse(ByteString.EMPTY),

        version = bc.version
      ),

      g ⇒ {
        val pk = KeyPair.Public(ByteBuffer wrap g.publicKey.toByteArray)

        def read[T](name: String, f: BasicContract ⇒ T): F[T] =
          Option(f(g)).fold[F[T]](F.raiseError(new IllegalArgumentException(s"Required field not found: $name")))(F.pure)

        for {
          idb ← read("id", _.id.toByteArray)
          id ← Key.fromBytes(idb)

          participantsRequired ← read("participantsRequired", _.participantsRequired)

          offerSeal ← read("offerSeal", _.offerSeal.toByteArray)

          participants ← Traverse[List].traverse(g.participants.toList){ p ⇒
            Key.fromBytes[F](p.id.toByteArray).map(_ -> Signature(
              KeyPair.Public(ByteBuffer wrap p.publicKey.toByteArray),
              ByteBuffer wrap p.signature.toByteArray
            ))
          }

          version ← read("version", _.version)
        } yield fluence.dataset.BasicContract(
          id = id,

          offer = fluence.dataset.BasicContract.Offer(
            participantsRequired = participantsRequired
          ),

          offerSeal = Signature(pk, ByteBuffer.wrap(offerSeal)),

          participants = participants.toMap,

          participantsSeal = Option(g.participantsSeal)
            .map(_.toByteArray)
            .map(ByteBuffer wrap _)
            .map(Signature(pk, _)),

          version = version
        )
      }
    )

}
