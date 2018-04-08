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

package fluence.dataset.node

import fluence.btree.common.merkle.MerkleRootCalculator
import fluence.btree.core.{Hash, Key}
import fluence.btree.protocol.BTreeRpc.{PutCallbacks, SearchCallback}
import fluence.btree.server.{Get, MerkleBTree, Put, Range}
import fluence.dataset.node.DatasetNodeStorage.DatasetChanged
import fluence.storage.KVStore
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import monix.reactive.Observable
import org.mockito.{ArgumentMatchers, Mockito}
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.mockito.MockitoSugar
import org.scalatest.{BeforeAndAfterEach, Matchers, WordSpec}

class DatasetNodeStorageSpec
    extends WordSpec with Matchers with MockitoSugar with ScalaFutures with BeforeAndAfterEach {

  private val get = mock[SearchCallback[Task]]
  private val put = mock[PutCallbacks[Task]]
  private val mBtree = mock[MerkleBTree]
  private val kvStore = mock[KVStore[Task, Long, Array[Byte]]]
  private val mrCalc = mock[MerkleRootCalculator]

  private val valGen = () ⇒ 5L
  private val onMRChange: DatasetChanged ⇒ Task[Unit] = dc ⇒ Task(()) // todo test this callback

  private val key1 = "k1".toKey
  private val key2 = "k2".toKey
  private val expValue1 = "val1".getBytes
  private val expValue2 = "val2".getBytes
  private val someError = new IllegalArgumentException("Some error")

  "DatasetNodeStorage.get" should {
    "raises fail" when {
      "bTreeIndex raised fail" in {
        Mockito
          .when(mBtree.get(ArgumentMatchers.any(classOf[Get])))
          .thenReturn(Task.raiseError(someError))

        val store = new DatasetNodeStorage(mBtree, kvStore, mrCalc, valGen, onMRChange)

        val result = store.get(get).failed.runAsync.futureValue
        result shouldBe someError
      }

      "kvStore raised fail" in {
        Mockito
          .when(mBtree.get(ArgumentMatchers.any(classOf[Get])))
          .thenReturn(Task(Some(1L)))
        Mockito
          .when(kvStore.get(ArgumentMatchers.any(classOf[Long])))
          .thenReturn(Task.raiseError(someError))

        val store = new DatasetNodeStorage(mBtree, kvStore, mrCalc, valGen, onMRChange)

        val result = store.get(get).failed.runAsync.futureValue
        result shouldBe someError
      }

    }

    "key wasn't found key" in {
      Mockito
        .when(mBtree.get(ArgumentMatchers.any(classOf[Get])))
        .thenReturn(Task(None))

      val store = new DatasetNodeStorage(mBtree, kvStore, mrCalc, valGen, onMRChange)

      val result = store.get(get).runAsync.futureValue
      result shouldBe None
    }

    "returns value" in {
      Mockito
        .when(mBtree.get(ArgumentMatchers.any(classOf[Get])))
        .thenReturn(Task(Some(1L)))
      Mockito
        .when(kvStore.get(ArgumentMatchers.eq(1L)))
        .thenReturn(Task(Some(expValue1)))

      val store = new DatasetNodeStorage(mBtree, kvStore, mrCalc, valGen, onMRChange)

      val result = store.get(get).runAsync.futureValue
      result.get shouldBe expValue1
    }
  }

  "DatasetNodeStorage.range" should {
    "raises fail" when {
      "bTreeIndex raised fail" in {
        Mockito
          .when(mBtree.range(ArgumentMatchers.any(classOf[Range])))
          .thenReturn(Observable.raiseError(someError))

        val store = new DatasetNodeStorage(mBtree, kvStore, mrCalc, valGen, onMRChange)

        val result = store.range(get).failed.headL.runAsync.futureValue
        result shouldBe someError
      }

      "kvStore raised fail" in {
        Mockito
          .when(mBtree.range(ArgumentMatchers.any(classOf[Range])))
          .thenReturn(Observable(key1 → 1L))
        Mockito
          .when(kvStore.get(ArgumentMatchers.any(classOf[Long])))
          .thenReturn(Task.raiseError(someError))

        val store = new DatasetNodeStorage(mBtree, kvStore, mrCalc, valGen, onMRChange)

        val result = store.range(get).failed.headL.runAsync.futureValue
        result shouldBe someError
      }

    }

    "key wasn't found key" in {
      Mockito
        .when(mBtree.range(ArgumentMatchers.any(classOf[Range])))
        .thenReturn(Observable.empty)

      val store = new DatasetNodeStorage(mBtree, kvStore, mrCalc, valGen, onMRChange)

      val result = store.range(get).toListL.runAsync.futureValue
      result shouldBe empty
    }

    "returns one value" in {
      Mockito
        .when(mBtree.range(ArgumentMatchers.any(classOf[Range])))
        .thenReturn(Observable(key1 → 1L))
      Mockito
        .when(kvStore.get(ArgumentMatchers.eq(1L)))
        .thenReturn(Task(Some(expValue1)))

      val store = new DatasetNodeStorage(mBtree, kvStore, mrCalc, valGen, onMRChange)

      val result = store.range(get).headL.runAsync.futureValue
      result shouldBe key1.bytes → expValue1
    }

    "returns many values" in {
      Mockito
        .when(mBtree.range(ArgumentMatchers.any(classOf[Range])))
        .thenReturn(Observable(key1 → 1L, key2 → 1L))
      Mockito
        .when(kvStore.get(ArgumentMatchers.anyLong()))
        .thenReturn(Task(Some(expValue1)), Task(Some(expValue2)))

      val store = new DatasetNodeStorage(mBtree, kvStore, mrCalc, valGen, onMRChange)

      val result = store.range(get).toListL.runAsync.futureValue
      result should have size 2
      result.head shouldBe key1.bytes → expValue1
      result.last shouldBe key2.bytes → expValue2
    }
  }

  "DatasetNodeStorage.put" should {
    "raises fail" when {
      "bTreeIndex raised fail" in {
        Mockito
          .when(mBtree.put(ArgumentMatchers.any(classOf[Put])))
          .thenReturn(Task.raiseError(someError))

        val store = new DatasetNodeStorage(mBtree, kvStore, mrCalc, valGen, onMRChange)

        val result = store.put(10L, put, expValue1).failed.runAsync.futureValue
        result shouldBe someError
      }

      "kvStore raised fail" in {
        Mockito
          .when(mBtree.put(ArgumentMatchers.any(classOf[Put])))
          .thenReturn(Task(1L))
        Mockito
          .when(kvStore.get(1L))
          .thenReturn(Task.raiseError(someError))
        Mockito
          .when(kvStore.put(1L, expValue1))
          .thenReturn(Task.raiseError(someError))

        val store = new DatasetNodeStorage(mBtree, kvStore, mrCalc, valGen, onMRChange)

        val result = store.put(10L, put, expValue1).failed.runAsync.futureValue
        result shouldBe someError
      }

      "onMRChange raised fail" in {
        Mockito
          .when(mBtree.put(ArgumentMatchers.any(classOf[Put])))
          .thenReturn(Task(1L))
        Mockito
          .when(kvStore.get(1L))
          .thenReturn(Task.raiseError(someError))
        Mockito
          .when(kvStore.put(1L, expValue1))
          .thenReturn(Task.raiseError(someError))
        val onMRChangeWithError: Array[Byte] ⇒ Task[Unit] = _ ⇒ Task.raiseError(someError)

        val store = new DatasetNodeStorage(mBtree, kvStore, mrCalc, valGen, onMRChange)

        val result = store.put(10L, put, expValue1).failed.runAsync.futureValue
        result shouldBe someError
      }
    }

    "put new value" in {
      Mockito
        .when(mBtree.put(ArgumentMatchers.any(classOf[Put])))
        .thenReturn(Task(1L))
      Mockito
        .when(kvStore.get(1L))
        .thenReturn(Task(None))
      Mockito
        .when(kvStore.put(1L, expValue1))
        .thenReturn(Task(()))
      Mockito
        .when(mBtree.getMerkleRoot)
        .thenReturn(Task(Hash("hash".getBytes)))

      val store = new DatasetNodeStorage(mBtree, kvStore, mrCalc, valGen, onMRChange)

      val result = store.put(10L, put, expValue1).runAsync.futureValue
      result shouldBe None
    }

    "returns old value if old value was overwritten" in {
      val oldValue = "old".getBytes
      Mockito
        .when(mBtree.put(ArgumentMatchers.any(classOf[Put])))
        .thenReturn(Task(1L))
      Mockito
        .when(kvStore.get(1L))
        .thenReturn(Task(Some(oldValue)))
      Mockito
        .when(kvStore.put(1L, expValue1))
        .thenReturn(Task(()))
      Mockito
        .when(mBtree.getMerkleRoot)
        .thenReturn(Task(Hash("hash".getBytes)))

      val store = new DatasetNodeStorage(mBtree, kvStore, mrCalc, valGen, onMRChange)

      val result = store.put(10L, put, expValue1).runAsync.futureValue
      result.get shouldBe oldValue
    }

  }

  private implicit class Str2Key(str: String) {
    def toKey: Key = Key(str.getBytes)
  }

  override protected def afterEach(): Unit = {
    Mockito.reset(get, mBtree, kvStore, mrCalc)
    super.afterEach()
  }
}
