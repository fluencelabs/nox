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

package fluence.contract.grpc

import cats.instances.list._
import cats.instances.option._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{MonadError, Traverse}
import com.google.protobuf.ByteString
import fluence.codec.{Codec, PureCodec}
import fluence.codec.pb.ProtobufCodecs._
import fluence.contract
import fluence.contract.BasicContract.ExecutionState
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{PubKeyAndSignature, Signature}
import fluence.kad.protocol.Key
import scodec.bits.ByteVector

import scala.language.higherKinds

object BasicContractCodec {

  /**
   * Codec for convert BasicContract into grpc representation. Checks all signatures as well.
   */
  implicit def codec[F[_]](
    implicit F: MonadError[F, Throwable],
  ): Codec[F, contract.BasicContract, BasicContract] = {

    val keyC = PureCodec.codec[Key, ByteString].toCodec[F]
    val strVec = PureCodec.codec[ByteVector, ByteString].toCodec[F]
    val pubKeyCV: Codec[F, KeyPair.Public, ByteVector] = Codec.pure(_.value, KeyPair.Public)
    val pubKeyC = pubKeyCV andThen strVec
    val optStrVecC = PureCodec.codec[Option[ByteVector], Option[ByteString]].toCodec[F]

    val encode: contract.BasicContract ⇒ F[BasicContract] =
      (bc: contract.BasicContract) ⇒
        for {
          idBs ← keyC.encode(bc.id)
          pubKBs ← pubKeyC.encode(bc.publicKey)

          participantsBs ← Traverse[List].traverse(bc.participants.toList) {
            case (pK: Key, pS: PubKeyAndSignature) ⇒
              for {
                pkBs ← keyC.encode(pK)
                pubKBs ← pubKeyC.encode(pS.publicKey)
                signBs ← strVec.encode(pS.signature.sign)
              } yield Participant(id = pkBs, publicKey = pubKBs, signature = signBs)
          }

          offSBs ← strVec.encode(bc.offerSeal.sign)
          participantsSealBs ← optStrVecC.encode(bc.participantsSeal.map(_.sign))
          executionSealBs ← strVec.encode(bc.executionSeal.sign)

          merkleRootBs ← strVec.encode(bc.executionState.merkleRoot)
        } yield
          BasicContract(
            id = idBs,
            publicKey = pubKBs,
            offer = Some(
              new BasicContractOffer(participantsRequired = bc.offer.participantsRequired)
            ),
            offerSeal = offSBs,
            participants = participantsBs,
            participantsSeal = participantsSealBs.getOrElse(ByteString.EMPTY),
            version = bc.executionState.version,
            merkleRoot = merkleRootBs,
            executionSeal = executionSealBs
        )

    val decode: BasicContract ⇒ F[contract.BasicContract] =
      grpcContact ⇒ {
        def read[T](name: String, f: BasicContract ⇒ T): F[T] =
          Option(f(grpcContact))
            .fold[F[T]](F.raiseError(new IllegalArgumentException(s"Required field not found: $name")))(F.pure)

        def readFromOpt[T](name: String, f: BasicContract ⇒ Option[T]): F[T] =
          f(grpcContact)
            .fold[F[T]](F.raiseError(new IllegalArgumentException(s"Required field not found: $name")))(F.pure)

        def readParticipantsSeal: F[Option[ByteVector]] =
          Option(grpcContact.participantsSeal)
            .filter(_.size() > 0)
            .fold(F.pure(Option.empty[ByteVector]))(sl ⇒ strVec.decode(sl).map(Option(_)))

        for {
          pubKey ← read("publicKey", _.publicKey).flatMap(pubKeyC.decode)

          id ← read("id", _.id).flatMap(keyC.decode)

          participantsRequired ← readFromOpt("participantsRequired", _.offer.map(_.participantsRequired))

          offerSeal ← read("offerSeal", _.offerSeal).flatMap(strVec.decode)

          participants ← Traverse[List].traverse(grpcContact.participants.toList) { participant ⇒
            for {
              key ← keyC.decode(participant.id)
              pubK ← pubKeyC.decode(participant.publicKey)
              sign ← strVec.decode(participant.signature)
            } yield key -> PubKeyAndSignature(pubK, Signature(sign))
          }

          version ← read("version", _.version)

          participantsSealOpt ← readParticipantsSeal

          merkleRoot ← read("merkleRoot", _.merkleRoot).flatMap(strVec.decode)

          execSeal ← read("executionSeal", _.executionSeal).flatMap(strVec.decode)

          deserializedContract = contract.BasicContract(
            id = id,
            publicKey = pubKey,
            offer = fluence.contract.BasicContract.Offer(
              participantsRequired = participantsRequired
            ),
            offerSeal = Signature(offerSeal),
            participants = participants.toMap,
            participantsSeal = participantsSealOpt.map(Signature(_)),
            executionState = ExecutionState(
              version = version,
              merkleRoot = merkleRoot
            ),
            executionSeal = Signature(execSeal)
          )
        } yield deserializedContract
      }

    Codec(encode, decode)
  }

}
