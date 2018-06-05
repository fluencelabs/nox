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

import java.time.Clock

import cats.data.EitherT
import cats.effect.IO
import com.typesafe.config.{ConfigException, ConfigFactory}
import fluence.codec.PureCodec
import fluence.codec.bits.BitsCodecs._
import fluence.contract.BasicContract
import fluence.contract.node.cache.ContractRecord
import fluence.crypto.KeyPair
import fluence.crypto.signature.{PubKeyAndSignature, Signature}
import fluence.kad.protocol.Key
import fluence.kvstore.{InMemoryKVStore, KVStore, ReadWriteKVStore, StoreError}
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

class ContractsCacheStoreSpec extends WordSpec with Matchers {

  private val config = ConfigFactory.load()
  private val clock = Clock.systemUTC()

  implicit val idCodec: PureCodec[Array[Byte], Array[Byte]] = PureCodec.identityBijection

  val inMemStore: ReadWriteKVStore[ByteVector, Array[Byte]] = new InMemoryKVStore[ByteVector, Array[Byte]]
  val storage: ReadWriteKVStore[Array[Byte], Array[Byte]] = KVStore.withCodecs(inMemStore)

  private val createKVStore: String ⇒ EitherT[IO, StoreError, ReadWriteKVStore[Array[Byte], Array[Byte]]] =
    name ⇒ EitherT.rightT(storage)

  "ContractsCacheStore" should {
    "fail on initialisation" when {
      "config unreachable" in {
        val emptyConfig = ConfigFactory.empty()

        val result = ContractsCacheStore(emptyConfig, createKVStore).value.unsafeRunSync()
        result.left.get shouldBe a[StoreError]
        result.left.get.getCause shouldBe a[ConfigException.Missing]
      }

      "kvStoreFactory failed" in {
        val createKVStoreWithFail: String ⇒ EitherT[IO, StoreError, ReadWriteKVStore[Array[Byte], Array[Byte]]] =
          _ ⇒ EitherT.leftT(StoreError("storage error!"))

        val result = ContractsCacheStore(config, createKVStoreWithFail).value.unsafeRunSync()
        result.left.get shouldBe a[StoreError]
        result.left.get.getMessage shouldBe "storage error!"
      }
    }

    "create store" when {
      "all is fine" in {
        val result = ContractsCacheStore(config, createKVStore).value.unsafeRunSync()
        result should not be null
      }
    }

    "performs all operations success" in {
      val store: ReadWriteKVStore[Key, ContractRecord[BasicContract]] =
        ContractsCacheStore(config, createKVStore).value.unsafeRunSync().right.get

      val seed = "seed".getBytes()
      val keyPair = KeyPair.fromBytes(seed, seed)
      import fluence.crypto.DumbCrypto.signAlgo
      val signer = signAlgo.signer(keyPair)

      val key1 = Key.fromKeyPair.unsafe(keyPair)
      val val1 = ContractRecord(BasicContract.offer[IO](key1, 2, signer).unsafeRunSync(), clock.instant())
      val participants = {
        val keypair: KeyPair = KeyPair.fromBytes(Array.emptyByteArray, Array.emptyByteArray)
        val nodeId: Key = Key.fromPublicKey.unsafe(keypair.publicKey)
        Map(nodeId → PubKeyAndSignature(keypair.publicKey, Signature(ByteVector.fromLong(100L))))
      }
      val val2 = ContractRecord(
        BasicContract.offer[IO](key1, 4, signer).unsafeRunSync().copy(participants = participants),
        clock.instant()
      )

      val get1 = store.get(key1).runUnsafe()
      get1 shouldBe None

      store.put(key1, val1).runUnsafe()

      val get2 = store.get(key1).runUnsafe()
      get2.get shouldBe val1

      store.put(key1, val2).runUnsafe()

      val get3 = store.get(key1).runUnsafe()
      get3.get shouldBe val2

    }

  }

}
