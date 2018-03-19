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

package fluence.node.core

import java.time.Instant

import cats.effect.IO
import cats.instances.list._
import cats.instances.option._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{ MonadError, Traverse }
import com.google.protobuf.ByteString
import com.typesafe.config.Config
import fluence.codec.Codec
import fluence.contract
import fluence.contract.BasicContract
import fluence.contract.node.cache.ContractRecord
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signature
import fluence.kad.protocol.Key
import fluence.node.persistence.{ BasicContractCache, Participant }
import fluence.storage.KVStore
import fluence.codec.pb.ProtobufCodecs._
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Factory for creating [[KVStore]] instance for caching contracts on the node side.
 */
object ContractsCacheStore {

  /** Creates [[fluence.codec.Codec]] instance for {{{ContractRecord[BasicContract]}}} and {{{BasicContractCache}}} */
  private def contractRec2ContractCacheCodec[F[_]](
    implicit
    F: MonadError[F, Throwable]
  ): Codec[F, ContractRecord[BasicContract], BasicContractCache] = {
    val keyC = Codec.codec[F, Key, ByteString]
    val strVec = Codec.codec[F, ByteVector, ByteString]

    val pubKeyCV: Codec[F, KeyPair.Public, ByteVector] = Codec.pure(_.value, KeyPair.Public)
    val pubKeyC = pubKeyCV andThen strVec

    val optStrVecC = Codec.codec[F, Option[ByteVector], Option[ByteString]]

    Codec(
      contractRec ⇒ {
        val bc = contractRec.contract
        for {
          idBs ← keyC.encode(bc.id)

          participantsBs ← Traverse[List].traverse(bc.participants.toList) {
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
          executionSealBs ← optStrVecC.encode(bc.executionSeal.map(_.sign))

          merkleRootBs ← strVec.encode(bc.executionState.merkleRoot)

        } yield BasicContractCache(
          id = idBs,
          publicKey = pkBs,

          participantsRequired = bc.offer.participantsRequired,

          offerSeal = offSBs,

          participants = participantsBs,

          participantsSeal = participantsSealBs.getOrElse(ByteString.EMPTY),

          version = bc.executionState.version,
          merkleRoot = merkleRootBs,
          executionSeal = executionSealBs.getOrElse(ByteString.EMPTY),

          lastUpdated = contractRec.lastUpdated.toEpochMilli
        )
      },

      basicContractCache ⇒ {
        def read[T](name: String, f: BasicContractCache ⇒ T): F[T] =
          Option(f(basicContractCache))
            .fold[F[T]](F.raiseError(new IllegalArgumentException(s"Required field not found: $name")))(F.pure)

        def readParticipantsSeal: F[Option[ByteVector]] =
          Option(basicContractCache.participantsSeal)
            .filter(_.size() > 0)
            .fold(F.pure(Option.empty[ByteVector]))(sl ⇒ strVec.decode(sl).map(Option(_)))

        for {
          pk ← pubKeyC.decode(basicContractCache.publicKey)

          idb ← read("id", _.id)
          id ← keyC.decode(idb)

          participantsRequired ← read("participantsRequired", _.participantsRequired)

          offerSealBS ← read("offerSeal", _.offerSeal)
          offerSealVec ← strVec.decode(offerSealBS)

          participants ← Traverse[List].traverse(basicContractCache.participants.toList) { p ⇒
            for {
              k ← keyC.decode(p.id)
              kp ← pubKeyC.decode(p.publicKey)
              s ← strVec.decode(p.signature)
            } yield k -> Signature(kp, s)
          }

          version ← read("version", _.version)

          participantsSealOpt ← readParticipantsSeal

          merkleRootBS ← read("merkleRoot", _.merkleRoot)
          merkleRoot ← strVec.decode(merkleRootBS)

          execSeal ← optStrVecC.decode(toOption(basicContractCache.executionSeal))

          lastUpdated ← read("lastUpdated", _.lastUpdated)
        } yield ContractRecord(

          contract.BasicContract(
            id = id,

            offer = fluence.contract.BasicContract.Offer(
              participantsRequired = participantsRequired
            ),

            offerSeal = Signature(pk, offerSealVec),

            participants = participants.toMap,

            participantsSeal = participantsSealOpt
              .map(Signature(pk, _)),

            executionState = BasicContract.ExecutionState(
              version = version,
              merkleRoot = merkleRoot
            ),
            executionSeal = execSeal.map(Signature(pk, _))

          ),
          Instant.ofEpochMilli(lastUpdated)
        )
      }
    )
  }

  private def toOption[F[_]](byteStr: ByteString) = if (byteStr.isEmpty) None else Option(byteStr)

  /** Creates [[fluence.codec.Codec]] instance for {{{BasicContractCache}}} and {{{Array[Byte]}}} */
  private def contractCache2Bytes[F[_]](implicit F: MonadError[F, Throwable]): Codec[F, BasicContractCache, Array[Byte]] =
    Codec[F, BasicContractCache, Array[Byte]](
      bcc ⇒ F.pure(bcc.toByteArray),
      bytes ⇒ F.pure(BasicContractCache.parseFrom(bytes))
    )

  /**
   * Creates [[KVStore]] for caching contracts. Wraps 'binary store' with key and value codecs.
   *
   * @param config          Global typeSafe config
   * @param kvStoreFactory Takes storage string name and return binary KVStore
   * @return contract cache key/value Store
   */
  def apply[F[_]](
    config: Config,
    kvStoreFactory: String ⇒ IO[KVStore[F, Array[Byte], Array[Byte]]]
  )(implicit F: MonadError[F, Throwable]): IO[KVStore[F, Key, ContractRecord[BasicContract]]] =
    for {
      conf ← ContractsCacheConf.read(config)
      binStore ← kvStoreFactory(conf.dataDir)
    } yield {
      apply(binStore)
    }

  /**
   * Creates [[KVStore]] for caching contracts. Wraps 'binary store' with key and value codecs.
   *
   * @param contractCacheBinaryStore Task based key/value store for binary data
   * @return contract cache key/value Store
   */
  def apply[F[_]](
    contractCacheBinaryStore: KVStore[F, Array[Byte], Array[Byte]]
  )(implicit F: MonadError[F, Throwable]): KVStore[F, Key, ContractRecord[BasicContract]] = {
    import Key.bytesCodec

    implicit val contractRecord2BytesCodec: Codec[F, ContractRecord[BasicContract], Array[Byte]] =
      contractRec2ContractCacheCodec[F] andThen contractCache2Bytes[F]

    contractCacheBinaryStore
  }

}
