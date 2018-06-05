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

import cats.data.EitherT
import cats.effect.IO
import cats.instances.list._
import cats.instances.option._
import cats.syntax.compose._
import cats.syntax.functor._
import cats.{Monad, Traverse}
import com.google.protobuf.ByteString
import com.typesafe.config.Config
import fluence.codec.bits.BitsCodecs._
import fluence.codec.pb.ProtobufCodecs._
import fluence.codec.{CodecError, PureCodec}
import fluence.contract
import fluence.contract.BasicContract
import fluence.contract.node.cache.ContractRecord
import fluence.crypto.KeyPair
import fluence.crypto.signature.{PubKeyAndSignature, Signature}
import fluence.kad.protocol.Key
import fluence.kvstore.{KVStore, ReadWriteKVStore, StoreError}
import fluence.node.persistence.{BasicContractCache, Participant}
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Factory for creating [[KVStore]] instance for caching contracts on the node side.
 */
object ContractsCacheStore {

  /**
   * Creates [[fluence.codec.PureCodec]] instance for {{{ContractRecord[BasicContract]}}} and {{{BasicContractCache}}}
   * Don't need checking 'seals' because we store only valid contracts.
   */
  private def contractRec2ContractCacheCodec: PureCodec[ContractRecord[BasicContract], BasicContractCache] = {

    implicit val byteVec2byteStr: PureCodec[ByteVector, ByteString] =
      PureCodec[ByteVector, Array[Byte]] andThen PureCodec[Array[Byte], ByteString]

    val key2byteStr: PureCodec[Key, ByteString] =
      PureCodec[Key, Array[Byte]] andThen PureCodec[Array[Byte], ByteString]

    val pubKey2ByteStr: PureCodec[KeyPair.Public, ByteString] =
      PureCodec.build[KeyPair.Public, ByteVector]((p: KeyPair.Public) ⇒ p.value, KeyPair.Public) andThen byteVec2byteStr

    val opByteVec2opByteStr: PureCodec[Option[ByteVector], Option[ByteString]] =
      PureCodec[Option[ByteVector], Option[ByteString]]

    val serializer = new PureCodec.Func[ContractRecord[BasicContract], BasicContractCache] {
      override def apply[F[_]: Monad](
        input: ContractRecord[BasicContract]
      ): EitherT[F, CodecError, BasicContractCache] = {
        val contract = input.contract
        for {
          idBs ← key2byteStr.direct[F](contract.id)
          pubKey ← byteVec2byteStr.direct[F](contract.publicKey.value)

          participants ← {
            val pList: F[List[Either[CodecError, Participant]]] = Traverse[List]
              .traverse[F, (Key, PubKeyAndSignature), Either[CodecError, Participant]](contract.participants.toList) {
                case (pk, ps) ⇒ {
                  for {
                    pId ← key2byteStr.direct[F](pk)
                    pPubKey ← pubKey2ByteStr.direct[F](ps.publicKey)
                    pSignature ← byteVec2byteStr.direct[F](ps.signature.sign)
                  } yield Participant(id = pId, publicKey = pPubKey, signature = pSignature)
                }.value
              }

            EitherT(pList.map(list2either))
          }

          offSBs ← byteVec2byteStr.direct[F](contract.offerSeal.sign)

          participantsSealBs ← opByteVec2opByteStr.direct[F](contract.participantsSeal.map(_.sign))
          executionSealBs ← byteVec2byteStr.direct[F](contract.executionSeal.sign)

          merkleRootBs ← byteVec2byteStr.direct[F](contract.executionState.merkleRoot)

        } yield
          BasicContractCache(
            id = idBs,
            publicKey = pubKey,
            participantsRequired = contract.offer.participantsRequired,
            offerSeal = offSBs,
            participants = participants,
            participantsSeal = participantsSealBs.getOrElse(ByteString.EMPTY),
            version = contract.executionState.version,
            merkleRoot = merkleRootBs,
            executionSeal = executionSealBs,
            lastUpdated = input.lastUpdated.toEpochMilli
          )
      }
    }

    val deserializer = new PureCodec.Func[BasicContractCache, ContractRecord[BasicContract]] {
      override def apply[F[_]: Monad](
        input: BasicContractCache
      ): EitherT[F, CodecError, ContractRecord[BasicContract]] = {

        def read[T](name: String, fn: BasicContractCache ⇒ T): EitherT[F, CodecError, T] =
          EitherT.fromOption(Option(fn(input)), CodecError(s"Required field is not found: $name"))

        def readParticipantsSeal: EitherT[F, CodecError, Option[ByteVector]] =
          Option(input.participantsSeal)
            .filterNot(_.isEmpty)
            .fold(EitherT.rightT[F, CodecError](Option.empty[ByteVector]))(
              sl ⇒ byteVec2byteStr.inverse[F](sl).map(Option(_))
            )

        for {
          pubKey ← pubKey2ByteStr.inverse[F](input.publicKey)

          idb ← read("id", _.id)
          id ← key2byteStr.inverse[F](idb)

          participantsRequired ← read("participantsRequired", _.participantsRequired)

          offerSealBS ← read("offerSeal", _.offerSeal)
          offerSealVec ← byteVec2byteStr.inverse[F](offerSealBS)

          participants ← {
            val pList = Traverse[List]
              .traverse[F, Participant, Either[CodecError, (Key, PubKeyAndSignature)]](input.participants.toList) {
                participant ⇒
                  {
                    for {
                      pId ← key2byteStr.inverse[F](participant.id)
                      pPubKey ← pubKey2ByteStr.inverse[F](participant.publicKey)
                      pSignature ← byteVec2byteStr.inverse[F](participant.signature)
                    } yield pId -> PubKeyAndSignature(pPubKey, Signature(pSignature))
                  }.value
              }

            EitherT(pList.map(list2either))
          }

          version ← read("version", _.version)

          participantsSealOpt ← readParticipantsSeal

          merkleRootBS ← read("merkleRoot", _.merkleRoot)
          merkleRoot ← byteVec2byteStr.inverse[F](merkleRootBS)

          execSeal ← byteVec2byteStr.inverse[F](input.executionSeal)

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
    }

    PureCodec.build(serializer, deserializer)
  }

  /** Creates [[fluence.codec.PureCodec]] instance for {{{BasicContractCache}}} and {{{Array[Byte]}}} */
  private def contractCache2Bytes: PureCodec[BasicContractCache, Array[Byte]] = {
    PureCodec.build(
      bcc ⇒ bcc.toByteArray,
      bytes ⇒ BasicContractCache.parseFrom(bytes)
    )
  }

  private def list2either[A, B](list: List[Either[A, B]]): Either[A, List[B]] =
    list.foldRight(Right(Nil): Either[A, List[B]]) { (either, acc) ⇒
      for (xs ← acc.right; x ← either.right) yield x :: xs
    }

  /**
   * Creates [[KVStore]] for caching contracts. Wraps 'binary store' with key and value codecs.
   *
   * @param config          Global typeSafe config
   * @param kvStoreFactory Factory for creating RocksDbKVStore instances
   * @return contract cache key/value Store
   */
  def apply[Store <: ReadWriteKVStore[Array[Byte], Array[Byte]]](
    config: Config,
    kvStoreFactory: String ⇒ EitherT[IO, StoreError, Store]
  ): EitherT[IO, StoreError, ReadWriteKVStore[Key, ContractRecord[BasicContract]]] =
    for {
      conf ← EitherT(ContractsCacheConf.read(config).attempt).leftMap(StoreError(_))
      binStore ← kvStoreFactory(conf.dataDir)
    } yield {
      apply(binStore)
    }

  /**
   * Creates [[KVStore]] for caching contracts. Wraps 'binary store' with key and value codecs.
   *
   * @param config          Global typeSafe config
   * @param kvStoreFactory Factory for creating RocksDbKVStore instances
   * @return contract cache key/value Store
   */
  @deprecated(
    "Use 'apply' version with EitherT return value instead. Will be removed when all API will be 'EitherT compatible'."
  )
  def applyOld[Store <: ReadWriteKVStore[Array[Byte], Array[Byte]]](
    config: Config,
    kvStoreFactory: String ⇒ EitherT[IO, StoreError, Store]
  ): IO[ReadWriteKVStore[Key, ContractRecord[BasicContract]]] =
    apply(config, kvStoreFactory).value.flatMap(IO.fromEither)

  /**
   * Creates [[KVStore]] for caching contracts. Wraps 'binary store' with key and value codecs.
   *
   * @param contractCacheBinaryStore Task based key/value store for binary data
   * @return contract cache key/value Store
   */
  def apply[Store <: ReadWriteKVStore[Array[Byte], Array[Byte]]](
    contractCacheBinaryStore: Store
  ): ReadWriteKVStore[Key, ContractRecord[BasicContract]] =
    KVStore
      .withCodecs(contractCacheBinaryStore)(
        Key.bytesCodec,
        contractRec2ContractCacheCodec andThen contractCache2Bytes
      )

}
