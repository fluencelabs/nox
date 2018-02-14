/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.dataset.grpc

import cats.{ MonadError, Traverse }
import cats.syntax.functor._
import cats.syntax.flatMap._
import com.google.protobuf.ByteString
import fluence.codec.Codec
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signature
import fluence.kad.protocol.Key
import cats.instances.list._
import cats.instances.option._
import fluence.transport.grpc.GrpcCodecs._
import scodec.bits.ByteVector

import scala.language.higherKinds

object BasicContractCodec {

  implicit def codec[F[_]](implicit F: MonadError[F, Throwable]): Codec[F, fluence.dataset.BasicContract, BasicContract] =
    {
      val keyC = Codec.codec[F, Key, ByteString]
      val strVec = Codec.codec[F, ByteVector, ByteString]

      val pubKeyCV: Codec[F, KeyPair.Public, ByteVector] = Codec.pure(_.value, KeyPair.Public)
      val pubKeyC = pubKeyCV andThen strVec

      val optStrVecC = Codec.codec[F, Option[ByteVector], Option[ByteString]]

      Codec(
        bc ⇒ for {
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

        } yield BasicContract(
          id = idBs,
          publicKey = pkBs,

          offer = Some(new BasicContractOffer(
            participantsRequired = bc.offer.participantsRequired
          )),

          offerSeal = offSBs,

          participants = participantsBs,

          participantsSeal = participantsSealBs.getOrElse(ByteString.EMPTY),

          version = bc.version
        ),

        g ⇒ {
          def read[T](name: String, f: BasicContract ⇒ T): F[T] =
            Option(f(g)).fold[F[T]](F.raiseError(new IllegalArgumentException(s"Required field not found: $name")))(F.pure)

          def readFromOpt[T](name: String, f: BasicContract ⇒ Option[T]): F[T] =
            Option(f(g)).flatten.fold[F[T]](F.raiseError(new IllegalArgumentException(s"Required field not found: $name")))(F.pure)

          def readParticipantsSeal: F[Option[ByteVector]] =
            Option(g.participantsSeal)
              .filter(_.size() > 0)
              .fold(F.pure(Option.empty[ByteVector]))(sl ⇒ strVec.decode(sl).map(Option(_)))

          for {
            pk ← pubKeyC.decode(g.publicKey)

            idb ← read("id", _.id)
            id ← keyC.decode(idb)

            participantsRequired ← readFromOpt("participantsRequired", _.offer.map(_.participantsRequired))

            offerSealBS ← read("offerSeal", _.offerSeal)
            offerSealVec ← strVec.decode(offerSealBS)

            participants ← Traverse[List].traverse(g.participants.toList){ p ⇒
              for {
                k ← keyC.decode(p.id)
                kp ← pubKeyC.decode(p.publicKey)
                s ← strVec.decode(p.signature)
              } yield k -> Signature(kp, s)
            }

            version ← read("version", _.version)

            participantsSealOpt ← readParticipantsSeal
          } yield fluence.dataset.BasicContract(
            id = id,

            offer = fluence.dataset.BasicContract.Offer(
              participantsRequired = participantsRequired
            ),

            offerSeal = Signature(pk, offerSealVec),

            participants = participants.toMap,

            participantsSeal = participantsSealOpt
              .map(Signature(pk, _)),

            version = version
          )
        }
      )
    }

}
