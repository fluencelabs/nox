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

package fluence.btree.client

import cats.Id
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.btree.core.{Hash, Key}
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.cipher.NoOpCrypt
import fluence.crypto.hash.TestCryptoHasher
import monix.eval.Task
import monix.execution.ExecutionModel
import monix.execution.schedulers.TestScheduler
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

import scala.collection.Searching.{Found, InsertionPoint}
import scala.concurrent.duration.{FiniteDuration, _}

class MerkleBTreeClientSpec extends WordSpec with Matchers with ScalaFutures {

  private val signAlgo = Ecdsa.signAlgo
  private val keyPair = signAlgo.generateKeyPair().value.right.get
  private val signer = signAlgo.signer(keyPair)
  private val checker = signAlgo.checker(keyPair.publicKey)

  implicit class Str2Key(str: String) {
    def toKey: Key = Key(str.getBytes)
  }

  private implicit class Str2Hash(str: String) {
    def toHash: Hash = Hash(str.getBytes)
  }

  private val testHasher = TestCryptoHasher.map(Hash(_))

  val key1 = "k1"
  val key2 = "k2"
  val key3 = "k3"
  val key4 = "k4"
  val key5 = "k5"

  val val1Hash = "v1-cs"
  val val2Hash = "v2-cs"
  val val3Hash = "v3-cs"
  val val4Hash = "v4-cs"
  val val5Hash = "v5-cs"

  "getCmd" should {
    "returns error" when {

      "server request 'nextChildIndex' is in invalid" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createClient("H<H<k1v1-cs>>")
        val getCallbacks = wait(client.initGet(key1))
        val childChecksums = Array("H<H<k1v1>H<k2v2>>".toHash, "H<H<k3v3>H<k4v4>>".toHash)
        val result = wait(
          getCallbacks
            .nextChildIndex(Array("unexpected key returned from server".toKey), childChecksums)
            .map(_ ⇒ ())
            .failed
        )

        result.getMessage should startWith("Checksum of branch didn't pass verifying")
      }

      "server request 'submitLeaf' is in invalid" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createClient("H<H<k1v1>H<k2v2-cs>>")
        val getCallbacks = wait(client.initGet(key1))

        val result = wait(
          getCallbacks
            .submitLeaf(
              Array(key1.toKey, "unexpected key returned from server".toKey),
              Array(val1Hash.toHash, val2Hash.toHash)
            )
            .map(_ ⇒ ())
            .failed
        )

        result.getMessage should startWith("Checksum of leaf didn't pass verifying")
      }

    }

    "returns None" when {
      "key isn't found" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClient("H<H<k1v1-cs>>")
        val getCallbacks = wait(client.initGet(key2))

        val result = wait(getCallbacks.submitLeaf(Array(key1.toKey), Array(val1Hash.toHash)))

        result shouldBe InsertionPoint(1)
      }
    }

    "returns founded result" when {
      "key was found in Root" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClient("H<H<k1v1-cs>>")
        val getCallbacks = wait(client.initGet(key1))

        val result = wait(getCallbacks.submitLeaf(Array(key1.toKey), Array(val1Hash.toHash)))

        result shouldBe Found(0)
      }

      "key was found at the second level of tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClient("H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k3v3-cs>H<k4v4-cs>>>")
        val getCallbacks = wait(client.initGet(key1))
        val childChecksums = Array("H<H<k1v1-cs>H<k2v2-cs>>".toHash, "H<H<k3v3-cs>H<k4v4-cs>>".toHash)

        val result = wait(
          for {
            _ ← getCallbacks.nextChildIndex(Array(key2.toKey), childChecksums)
            idx ← getCallbacks.submitLeaf(Array(key1.toKey, key2.toKey), Array(val1Hash.toHash, val2Hash.toHash))
          } yield idx
        )

        result shouldBe Found(0)
      }
    }
  }

  "put" should {
    "returns error" when {
      "server request 'nextChildIndex' is in invalid" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createClient("H<H<k1v1>>")
        val putCallbacks = wait(client.initPut(key1, val1Hash.toHash, 0L))

        val childChecksums = Array("H<H<k1v1>H<k2v2>>".toHash, "H<H<k3v3>H<k4v4>>".toHash)
        val result =
          wait(putCallbacks.nextChildIndex(Array("unexpected key returned from server".toKey), childChecksums).failed)

        result.getMessage should startWith("Checksum of branch didn't pass verifying")
      }

      "server request 'putDetails' is in invalid" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createClient("H<H<k1v1>H<k2v2>>")
        val putCallbacks = wait(client.initPut(key1, val1Hash.toHash, 0L))

        val result = wait(
          putCallbacks
            .putDetails(
              Array(key1.toKey, "unexpected key returned from server".toKey),
              Array(val1Hash.toHash, val2Hash.toHash)
            )
            .failed
        )

        result.getMessage should startWith("Checksum of leaf didn't pass verifying")
      }

      "server request 'verifyChanges' is in invalid" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClient("H<H<k1v1-cs>>")
        val version = 0L
        val invalidNewMRoot = "H<H<k1v1-cs>H<k2v2-cs>> broken".toHash

        val putCallbacks = wait(client.initPut(key2, val2Hash.toHash, version))

        val result = wait(
          (for {
            _ ← putCallbacks.putDetails(Array(key1.toKey), Array(val1Hash.toHash))
            signed ← putCallbacks.verifyChanges(invalidNewMRoot, wasSplitting = false)
            _ ← putCallbacks.changesStored()
          } yield signed).failed
        )
        result.getMessage should startWith("Server 'put response' didn't pass verifying for state")
      }

    }

    "put new key/value to tree" when {
      "key ins't present in tree (root inserting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClient("H<H<k1v1-cs>>")
        val version = 0L
        val newVersion = ByteVector.fromLong(version + 1)
        val newMRoot = "H<H<k1v1-cs>H<k2v2-cs>>".toHash

        val putCallbacks = wait(client.initPut(key2, val2Hash.toHash, version))

        val signedState = wait(
          for {
            _ ← putCallbacks.putDetails(Array(key1.toKey), Array(val1Hash.toHash))
            signed ← putCallbacks.verifyChanges(newMRoot, wasSplitting = false)
            _ ← putCallbacks.changesStored()
          } yield signed
        )
        checker.check[Id](signedState, newVersion ++ newMRoot.asByteVector).value.right.get shouldBe ()
      }

      "key was found in tree (root inserting) " in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClient("H<H<k1v1-cs>>")
        val version = 0L
        val newVersion = ByteVector.fromLong(version + 1)
        val newMRoot = "H<H<k1v2-cs>>".toHash

        val putCallbacks = wait(client.initPut(key1, val2Hash.toHash, version))

        val signedState = wait(
          for {
            _ ← putCallbacks.putDetails(Array(key1.toKey), Array(val1Hash.toHash))
            signed ← putCallbacks.verifyChanges(newMRoot, wasSplitting = false)
            _ ← putCallbacks.changesStored()
          } yield signed
        )

        checker.check[Id](signedState, newVersion ++ newMRoot.asByteVector).value.right.get shouldBe ()
      }

      "key ins't present in tree (second level inserting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClient("H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k4v4-cs>H<k5v5-cs>>>")
        val version = 0L
        val newVersion = ByteVector.fromLong(version + 1)
        val newMRoot = "H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k3v3-cs>H<k4v4-cs>H<k5v5-cs>>>".toHash
        val childChecksums = Array("H<H<k1v1-cs>H<k2v2-cs>>".toHash, "H<H<k4v4-cs>H<k5v5-cs>>".toHash)

        val putCallbacks = wait(client.initPut(key3, val3Hash.toHash, version))

        val signedState = wait(
          for {
            _ ← putCallbacks.nextChildIndex(Array(key2.toKey), childChecksums)
            _ ← putCallbacks.putDetails(Array(key4.toKey, key5.toKey), Array(val4Hash.toHash, val5Hash.toHash))
            signed ← putCallbacks.verifyChanges(newMRoot, wasSplitting = false)
            _ ← putCallbacks.changesStored()
          } yield signed
        )

        checker.check[Id](signedState, newVersion ++ newMRoot.asByteVector).value.right.get shouldBe ()
      }

      "key was present in tree (second level inserting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createClient("H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k4v4-cs>H<k5v5-cs>>>")
        val version = 0L
        val newVersion = ByteVector.fromLong(version + 1)
        val newMRoot = "H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k4v3-cs>H<k5v5-cs>>>".toHash
        val childChecksums = Array("H<H<k1v1-cs>H<k2v2-cs>>".toHash, "H<H<k4v4-cs>H<k5v5-cs>>".toHash)

        val putCallbacks = wait(client.initPut(key4, val3Hash.toHash, version))

        val signedState = wait(
          for {
            _ ← putCallbacks.nextChildIndex(Array(key2.toKey), childChecksums)
            _ ← putCallbacks.putDetails(Array(key4.toKey, key5.toKey), Array(val4Hash.toHash, val5Hash.toHash))
            signed ← putCallbacks.verifyChanges(newMRoot, wasSplitting = false)
            _ ← putCallbacks.changesStored()
          } yield signed
        )

        checker.check[Id](signedState, newVersion ++ newMRoot.asByteVector).value.right.get shouldBe ()
      }

      //   todo add case with tree rebalancing later

    }
  }

  private def wait[T](task: Task[T], time: FiniteDuration = 3.second)(implicit TS: TestScheduler): T = {
    val async = task.runAsync
    TS.tick(time)
    async.futureValue
  }

  private def createClient(mRoot: String): MerkleBTreeClient[String] = {
    MerkleBTreeClient[String](
      Some(ClientState(mRoot.toHash)),
      NoOpCrypt.forString,
      testHasher,
      signer
    )
  }

}
