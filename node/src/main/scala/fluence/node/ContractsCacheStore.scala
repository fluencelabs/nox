package fluence.node

import java.time.Instant

import com.google.protobuf.ByteString
import fluence.codec.Codec
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signature
import fluence.dataset.BasicContract
import fluence.dataset.node.contract.ContractRecord
import fluence.kad.protocol.Key
import fluence.node.persistence.{ BasicContractCache, Participant }
import fluence.storage.KVStore
import fluence.storage.rocksdb.RocksDbStore
import monix.eval.Task
import scodec.bits.ByteVector
import cats.instances.list._
import cats.instances.option._
import fluence.transport.grpc.GrpcCodecs._
import cats.{ MonadError, Traverse }
import cats.syntax.functor._
import cats.syntax.flatMap._

import scala.language.higherKinds

object ContractsCacheStore {

  private implicit def codec[F[_]](implicit F: MonadError[F, Throwable]): Codec[F, ContractRecord[fluence.dataset.BasicContract], BasicContractCache] =
    {
      val keyC = Codec.codec[F, Key, ByteString]
      val strVec = Codec.codec[F, ByteVector, ByteString]

      val pubKeyCV: Codec[F, KeyPair.Public, ByteVector] = Codec.pure(_.value, KeyPair.Public)
      val pubKeyC = pubKeyCV andThen strVec

      val optStrVecC = Codec.codec[F, Option[ByteVector], Option[ByteString]]

      Codec(
        bcc ⇒ {
          val bc = bcc.contract
          for {
            idBs ← keyC.encode(bc.id)

            participantsBs ← Traverse[List].traverse(bc.participants.toList){
              case (pk, ps) ⇒
                for {
                  pkBs ← keyC.encode(pk)
                  pubkBs ← pubKeyC.encode(ps.publicKey)
                  signBs ← strVec.encode(ps.sign)
                } yield Participant(id = pkBs, publicKey = pubkBs, signature = signBs)
            }

            pkBs ← pubKeyC.encode(bc.offerSeal.publicKey)
            offSBs ← strVec.encode(bc.offerSeal.sign)

            participantsSealBs ← optStrVecC.encode(bc.participantsSeal.map(_.sign))

          } yield BasicContractCache(
            id = idBs,
            publicKey = pkBs,

            participantsRequired = bc.offer.participantsRequired,

            offerSeal = offSBs,

            participants = participantsBs,

            participantsSeal = participantsSealBs.getOrElse(ByteString.EMPTY),

            version = bc.version,

            lastUpdated = bcc.lastUpdated.getEpochSecond
          )
        },

        g ⇒ {
          def read[T](name: String, f: BasicContractCache ⇒ T): F[T] =
            Option(f(g)).fold[F[T]](F.raiseError(new IllegalArgumentException(s"Required field not found: $name")))(F.pure)

          def readParticipantsSeal: F[Option[ByteVector]] =
            Option(g.participantsSeal)
              .filter(_.size() > 0)
              .fold(F.pure(Option.empty[ByteVector]))(sl ⇒ strVec.decode(sl).map(Option(_)))

          for {
            pk ← pubKeyC.decode(g.publicKey)

            idb ← read("id", _.id)
            id ← keyC.decode(idb)

            participantsRequired ← read("participantsRequired", _.participantsRequired)

            offerSealBS ← read("offerSeal", _.offerSeal)
            offerSealVec ← strVec.decode(offerSealBS)

            participants ← Traverse[List].traverse(g.participants.toList){ p ⇒
              println("read p, id length = " + p.id.size())
              for {
                k ← keyC.decode(p.id)
                kp ← pubKeyC.decode(p.publicKey)
                s ← strVec.decode(p.signature)
              } yield k -> Signature(kp, s)
            }

            version ← read("version", _.version)

            participantsSealOpt ← readParticipantsSeal

            lastUpdated ← read("lastUpdated", _.lastUpdated)
          } yield ContractRecord(
            fluence.dataset.BasicContract(
              id = id,

              offer = fluence.dataset.BasicContract.Offer(
                participantsRequired = participantsRequired
              ),

              offerSeal = Signature(pk, offerSealVec),

              participants = participants.toMap,

              participantsSeal = participantsSealOpt
                .map(Signature(pk, _)),

              version = version
            ),
            Instant.ofEpochSecond(lastUpdated)
          )
        }
      )
    }

  def apply(): KVStore[Task, Key, ContractRecord[BasicContract]] = {
    import Key.bytesCodec

    implicit val contractRecordCodec: Codec[Task, ContractRecord[BasicContract], Array[Byte]] =
      codec[Task] andThen Codec[Task, BasicContractCache, Array[Byte]](
        bcc ⇒ Task(bcc.toByteArray),
        bytes ⇒ Task(BasicContractCache.parseFrom(bytes))
      )

    RocksDbStore("fluence:contractsCache").get
  }
}
