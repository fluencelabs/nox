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
import cats.syntax.compose._
import cats.{MonadError, Traverse}
import com.google.protobuf.ByteString
import com.typesafe.config.Config
import fluence.codec.{Codec, PureCodec}
import fluence.codec.pb.ProtobufCodecs._
import fluence.contract
import fluence.contract.BasicContract
import fluence.contract.node.cache.ContractRecord
import fluence.kad.protocol.Key
import fluence.node.persistence.{BasicContractCache, Participant}
import fluence.storage.KVStore
import scodec.bits.ByteVector
import fluence.codec.bits.BitsCodecs._
import fluence.crypto.KeyPair
import fluence.crypto.signature.{PubKeyAndSignature, Signature}

import scala.language.higherKinds

/**
 * Factory for creating [[KVStore]] instance for caching contracts on the node side.
 */
object ContractsCacheStore {

  /**
   * Creates [[fluence.codec.Codec]] instance for {{{ContractRecord[BasicContract]}}} and {{{BasicContractCache}}}
   * Don't need checking 'seals' because we store only valid contracts.
   */
  private def contractRec2ContractCacheCodec[F[_]](
    implicit
    F: MonadError[F, Throwable]
  ): Codec[F, ContractRecord[BasicContract], BasicContractCache] = {
    val keyC = (PureCodec[Key, Array[Byte]] andThen PureCodec[Array[Byte], ByteString]).toCodec[F]
    implicit val strVecP: PureCodec[ByteVector, ByteString] =
      PureCodec[ByteVector, Array[Byte]] andThen PureCodec[Array[Byte], ByteString]
    val strVec = strVecP.toCodec[F]

    val pubKeyCV: Codec[F, KeyPair.Public, ByteVector] = Codec.pure(_.value, KeyPair.Public)
    val pubKeyC = pubKeyCV andThen strVec

    val optStrVecC = PureCodec[Option[ByteVector], Option[ByteString]].toCodec[F]

    val serializer: ContractRecord[BasicContract] ⇒ F[BasicContractCache] =
      contractRec ⇒ {
        val contract = contractRec.contract
        for {
          idBs ← keyC.encode(contract.id)
          pubKey ← strVec.encode(contract.publicKey.value)

          participantsBs ← Traverse[List].traverse(contract.participants.toList) {
            case (pk, ps) ⇒
              for {
                pkBs ← keyC.encode(pk)
                pubkBs ← pubKeyC.encode(ps.publicKey)
                signBs ← strVec.encode(ps.signature.sign)
              } yield Participant(id = pkBs, publicKey = pubkBs, signature = signBs)
          }

          offSBs ← strVec.encode(contract.offerSeal.sign)

          participantsSealBs ← optStrVecC.encode(contract.participantsSeal.map(_.sign))
          executionSealBs ← strVec.encode(contract.executionSeal.sign)

          merkleRootBs ← strVec.encode(contract.executionState.merkleRoot)

        } yield
          BasicContractCache(
            id = idBs,
            publicKey = pubKey,
            participantsRequired = contract.offer.participantsRequired,
            offerSeal = offSBs,
            participants = participantsBs,
            participantsSeal = participantsSealBs.getOrElse(ByteString.EMPTY),
            version = contract.executionState.version,
            merkleRoot = merkleRootBs,
            executionSeal = executionSealBs,
            lastUpdated = contractRec.lastUpdated.toEpochMilli
          )
      }

    val deserializer: BasicContractCache ⇒ F[ContractRecord[BasicContract]] =
      basicContractCache ⇒ {
        def read[T](name: String, f: BasicContractCache ⇒ T): F[T] =
          Option(f(basicContractCache))
            .fold[F[T]](F.raiseError(new IllegalArgumentException(s"Required field not found: $name")))(F.pure)

        def readParticipantsSeal: F[Option[ByteVector]] =
          Option(basicContractCache.participantsSeal)
            .filter(_.size() > 0)
            .fold(F.pure(Option.empty[ByteVector]))(sl ⇒ strVec.decode(sl).map(Option(_)))

        for {
          pubKey ← pubKeyC.decode(basicContractCache.publicKey)

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
            } yield k -> PubKeyAndSignature(kp, Signature(s))
          }

          version ← read("version", _.version)

          participantsSealOpt ← readParticipantsSeal

          merkleRootBS ← read("merkleRoot", _.merkleRoot)
          merkleRoot ← strVec.decode(merkleRootBS)

          execSeal ← strVec.decode(basicContractCache.executionSeal)

          lastUpdated ← read("lastUpdated", _.lastUpdated)
        } yield
          ContractRecord(
            contract.BasicContract(
              id = id,
              publicKey = pubKey,
              offer = fluence.contract.BasicContract.Offer(
                participantsRequired = participantsRequired
              ),
              offerSeal = Signature(offerSealVec),
              participants = participants.toMap,
              participantsSeal = participantsSealOpt.map(Signature(_)),
              executionState = BasicContract.ExecutionState(
                version = version,
                merkleRoot = merkleRoot
              ),
              executionSeal = Signature(execSeal)
            ),
            Instant.ofEpochMilli(lastUpdated)
          )
      }

    Codec(serializer, deserializer)
  }

  /** Creates [[fluence.codec.Codec]] instance for {{{BasicContractCache}}} and {{{Array[Byte]}}} */
  private def contractCache2Bytes[F[_]](
    implicit F: MonadError[F, Throwable]
  ): Codec[F, BasicContractCache, Array[Byte]] =
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

    implicit val keyBytesCodec: Codec[F, Array[Byte], Key] = bytesCodec.toCodec[F].swap

    implicit val contractRecord2BytesCodec: Codec[F, ContractRecord[BasicContract], Array[Byte]] =
      contractRec2ContractCacheCodec[F] andThen contractCache2Bytes[F]

    contractCacheBinaryStore
  }

}
