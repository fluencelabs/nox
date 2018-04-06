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

import cats.effect.IO
import com.typesafe.config.{ConfigException, ConfigFactory}
import fluence.codec.Codec
import fluence.contract.BasicContract
import fluence.contract.node.cache.ContractRecord
import fluence.crypto.SignAlgo
import fluence.crypto.keypair.KeyPair
import fluence.kad.protocol.Key
import fluence.storage.{KVStore, TrieMapKVStore}
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

class ContractsCacheStoreSpec extends WordSpec with Matchers {

  private val config = ConfigFactory.load()
  private val clock = Clock.systemUTC()

  implicit val strVec: Codec[IO, Array[Byte], ByteVector] = Codec.byteVectorArray
  implicit val idCodec: Codec[IO, Array[Byte], Array[Byte]] = Codec.identityCodec

  val inMemStore: KVStore[IO, ByteVector, Array[Byte]] = new TrieMapKVStore[IO, ByteVector, Array[Byte]]()
  val storage: KVStore[IO, Array[Byte], Array[Byte]] = KVStore.transform(inMemStore)

  private val createKVStore: String ⇒ IO[KVStore[IO, Array[Byte], Array[Byte]]] = name ⇒ IO(storage)

  "ContractsCacheStore" should {
    "fail on initialisation" when {
      "config unreachable" in {
        val emptyConfig = ConfigFactory.empty()

        val result = ContractsCacheStore(emptyConfig, createKVStore).attempt.unsafeRunSync()
        result.left.get shouldBe a[ConfigException.Missing]
      }

      "kvStoreFactory failed" in {
        val createKVStoreWithFail: String ⇒ IO[KVStore[IO, Array[Byte], Array[Byte]]] =
          name ⇒ IO.raiseError(new RuntimeException("storage error!"))

        val result = ContractsCacheStore(config, createKVStoreWithFail).attempt.unsafeRunSync()
        result.left.get shouldBe a[RuntimeException]
        result.left.get.getMessage shouldBe "storage error!"
      }
    }

    "create store" when {
      "all is fine" in {
        val result = ContractsCacheStore(config, createKVStore).unsafeRunSync()
        result should not be null
      }
    }

    "performs all operations success" in {
      val store: KVStore[IO, Key, ContractRecord[BasicContract]] =
        ContractsCacheStore(config, createKVStore).unsafeRunSync()

      val seed = "seed".getBytes()
      val keyPair = KeyPair.fromBytes(seed, seed)
      val signAlgo = SignAlgo.dumb
      val signer = signAlgo.signer(keyPair)

      val key1 = Key.fromKeyPair.unsafe(keyPair)
      val val1 = ContractRecord(BasicContract.offer[IO](key1, 2, signer).unsafeRunSync(), clock.instant())
      val val2 = ContractRecord(BasicContract.offer[IO](key1, 4, signer).unsafeRunSync(), clock.instant())

      val get1 = store.get(key1).attempt.unsafeRunSync()
      get1.left.get shouldBe a[NoSuchElementException]

      store.put(key1, val1).unsafeRunSync()

      val get2 = store.get(key1).attempt.unsafeRunSync()
      get2.right.get shouldBe val1

      store.put(key1, val2).unsafeRunSync()

      val get3 = store.get(key1).attempt.unsafeRunSync()
      get3.right.get shouldBe val2

    }

  }

}
