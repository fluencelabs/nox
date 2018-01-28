package fluence.btree.client

import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.btree.protocol.BTreeRpc
import fluence.crypto.cipher.NoOpCrypt
import fluence.crypto.hash.TestCryptoHasher
import monix.eval.Task
import monix.execution.ExecutionModel
import monix.execution.schedulers.TestScheduler
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{ Matchers, WordSpec }

import scala.concurrent.duration.{ FiniteDuration, _ }

class MerkleBTreeClientSpec extends WordSpec with Matchers with ScalaFutures {

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

  "get" should {
    "returns error" when {
      "btreeRpc returns error" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val error = new IllegalArgumentException("Broken pipe")
        val btreeRpc = new BTreeRpc[Task] {
          override def get(callbacks: BTreeRpc.GetCallbacks[Task]): Task[Unit] = Task.raiseError(error) // server returns error
          override def put(callbacks: BTreeRpc.PutCallbacks[Task]): Task[Unit] = ???
        }

        val client = createClient(btreeRpc, "H<H<k1>>")
        val exception = wait(client.get(key1).failed)
        exception.getMessage shouldBe error.getMessage
      }

      "verifying server NextChildSearchResponse was failed" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val btreeRpc = new BTreeRpc[Task] {
          override def get(callbacks: BTreeRpc.GetCallbacks[Task]): Task[Unit] = {
            val childChecksums = Array("H<H<k1v1>H<k2v2>>".getBytes, "H<H<k3v3>H<k4v4>>".getBytes)
            callbacks
              .nextChildIndex(Array("unexpected key returned from server".getBytes), childChecksums)
              .map(_ ⇒ ())
          }
          override def put(callbacks: BTreeRpc.PutCallbacks[Task]): Task[Unit] = ???
        }

        val client = createClient(btreeRpc, "H<H<k1v1-cs>>")
        val exception = wait(client.get(key1).failed)
        exception.getMessage should startWith("Checksum of branch didn't pass verifying")
      }

      "verifying server LeafResponse was failed" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val btreeRpc = new BTreeRpc[Task] {
          override def get(callbacks: BTreeRpc.GetCallbacks[Task]): Task[Unit] = {
            callbacks
              .submitLeaf(
                Array(key1.getBytes, "unexpected key returned from server".getBytes),
                Array(val1Hash.getBytes, val2Hash.getBytes)
              ).map(_ ⇒ ())
          }
          override def put(callbacks: BTreeRpc.PutCallbacks[Task]): Task[Unit] = ???
        }

        val client = createClient(btreeRpc, "H<H<k1v1>H<k2v2-cs>>")
        val exception = wait(client.get(key1).failed)
        exception.getMessage should startWith("Checksum of leaf didn't pass verifying")
      }

    }

    "returns None" when {
      "key isn't found" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val btreeRpc = new BTreeRpc[Task] {
          override def get(callbacks: BTreeRpc.GetCallbacks[Task]): Task[Unit] =
            callbacks.submitLeaf(Array(key1.getBytes), Array(val1Hash.getBytes)).map(_ ⇒ ())
          override def put(callbacks: BTreeRpc.PutCallbacks[Task]): Task[Unit] = ???
        }

        val client = createClient(btreeRpc, "H<H<k1v1-cs>>")
        val result = wait(client.get(key2))
        result shouldBe None
      }
    }

    "returns founded result" when {
      "key was found in Root" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val btreeRpc = new BTreeRpc[Task] {
          override def get(callbacks: BTreeRpc.GetCallbacks[Task]): Task[Unit] =
            callbacks.submitLeaf(Array(key1.getBytes), Array(val1Hash.getBytes)).map(_ ⇒ ())
          override def put(callbacks: BTreeRpc.PutCallbacks[Task]): Task[Unit] = ???
        }

        val client = createClient(btreeRpc, "H<H<k1v1-cs>>")
        wait(client.get(key1)).get shouldBe val1Hash.getBytes
      }

      "key was found at the second level of tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val btreeRpc = new BTreeRpc[Task] {
          override def get(callbacks: BTreeRpc.GetCallbacks[Task]): Task[Unit] = {
            val childChecksums = Array("H<H<k1v1-cs>H<k2v2-cs>>".getBytes, "H<H<k3v3-cs>H<k4v4-cs>>".getBytes)
            for {
              _ ← callbacks.nextChildIndex(Array(key2.getBytes), childChecksums)
              _ ← callbacks.submitLeaf(Array(key1.getBytes, key2.getBytes), Array(val1Hash.getBytes, val2Hash.getBytes))
            } yield ()
          }
          override def put(callbacks: BTreeRpc.PutCallbacks[Task]): Task[Unit] = ???
        }

        val client = createClient(btreeRpc, "H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k3v3-cs>H<k4v4-cs>>>")
        wait(client.get(key1)).get shouldBe val1Hash.getBytes
      }
    }
  }

  "put" should {
    "returns error" when {
      "btreeRpc returns error" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val error = new IllegalArgumentException("Broken pipe")
        val btreeRpc = new BTreeRpc[Task] {
          override def get(callbacks: BTreeRpc.GetCallbacks[Task]): Task[Unit] = ???
          override def put(callbacks: BTreeRpc.PutCallbacks[Task]): Task[Unit] =
            Task.raiseError(error)
        }

        val client = createClient(btreeRpc, "H<H<k1>>")
        val exception = wait(client.put(key1, val1Hash.getBytes).failed)
        exception.getMessage shouldBe error.getMessage
      }

      "verifying server NextChildSearchResponse was failed" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val btreeRpc = new BTreeRpc[Task] {
          override def get(callbacks: BTreeRpc.GetCallbacks[Task]): Task[Unit] = ???
          override def put(callbacks: BTreeRpc.PutCallbacks[Task]): Task[Unit] = {
            val childChecksums = Array("H<H<k1v1>H<k2v2>>".getBytes, "H<H<k3v3>H<k4v4>>".getBytes)
            callbacks.nextChildIndex(Array("unexpected key returned from server".getBytes), childChecksums).map(_ ⇒ ())
          }
        }

        val client = createClient(btreeRpc, "H<H<k1v1>>")
        val exception = wait(client.put(key1, val1Hash.getBytes).failed)
        exception.getMessage should startWith("Checksum of branch didn't pass verifying")
      }

      "verifying server LeafResponse was failed" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val btreeRpc = new BTreeRpc[Task] {
          override def get(callbacks: BTreeRpc.GetCallbacks[Task]): Task[Unit] = ???
          override def put(callbacks: BTreeRpc.PutCallbacks[Task]): Task[Unit] = {
            callbacks.putDetails(
              Array(key1.getBytes, "unexpected key returned from server".getBytes),
              Array(val1Hash.getBytes, val2Hash.getBytes)
            ).map(_ ⇒ ())
          }
        }

        val client = createClient(btreeRpc, "H<H<k1v1>H<k2v2>>")
        val exception = wait(client.put(key1, val1Hash.getBytes).failed)
        exception.getMessage should startWith("Checksum of leaf didn't pass verifying")
      }
    }

    "put new key/value to tree" when {
      "key ins't present in tree (root inserting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val btreeRpc = new BTreeRpc[Task] {
          override def get(callbacks: BTreeRpc.GetCallbacks[Task]): Task[Unit] = ???
          override def put(callbacks: BTreeRpc.PutCallbacks[Task]): Task[Unit] = {
            for {
              _ ← callbacks.putDetails(Array(key1.getBytes), Array(val1Hash.getBytes))
              _ ← callbacks.verifyChanges("H<H<k1v1-cs>H<k2v2-cs>>".getBytes, wasSplitting = false)
              _ ← callbacks.changesStored()
            } yield ()
          }
        }

        val client = createClient(btreeRpc, "H<H<k1v1-cs>>")
        val result = wait(client.put(key2, val2Hash.getBytes))
        result shouldBe None
      }

      "key was found in tree (root inserting) " in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val btreeRpc = new BTreeRpc[Task] {
          override def get(callbacks: BTreeRpc.GetCallbacks[Task]): Task[Unit] = ???
          override def put(callbacks: BTreeRpc.PutCallbacks[Task]): Task[Unit] = {
            for {
              _ ← callbacks.putDetails(Array(key1.getBytes), Array(val1Hash.getBytes))
              _ ← callbacks.verifyChanges("H<H<k1v2-cs>>".getBytes, wasSplitting = false)
              _ ← callbacks.changesStored()
            } yield ()
          }
        }

        val client = createClient(btreeRpc, "H<H<k1v1-cs>>")
        val result = wait(client.put(key1, val2Hash.getBytes))
        result.get shouldBe val1Hash.getBytes // val1 is old value that was rewrited
      }

      "key ins't present in tree (second level inserting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val btreeRpc = new BTreeRpc[Task] {
          override def get(callbacks: BTreeRpc.GetCallbacks[Task]): Task[Unit] = ???
          override def put(callbacks: BTreeRpc.PutCallbacks[Task]): Task[Unit] = {
            val childChecksums = Array("H<H<k1v1-cs>H<k2v2-cs>>".getBytes, "H<H<k4v4-cs>H<k5v5-cs>>".getBytes)
            for {
              _ ← callbacks.nextChildIndex(Array(key2.getBytes), childChecksums)
              _ ← callbacks.putDetails(Array(key4.getBytes, key5.getBytes), Array(val4Hash.getBytes, val5Hash.getBytes))
              _ ← callbacks.verifyChanges("H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k3v3-cs>H<k4v4-cs>H<k5v5-cs>>>".getBytes, wasSplitting = false)
              _ ← callbacks.changesStored()
            } yield ()
          }
        }

        val client = createClient(btreeRpc, "H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k4v4-cs>H<k5v5-cs>>>")
        val result = wait(client.put(key3, val3Hash.getBytes))
        result shouldBe None
      }

      "key was present in tree (second level inserting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val btreeRpc = new BTreeRpc[Task] {
          override def get(callbacks: BTreeRpc.GetCallbacks[Task]): Task[Unit] = ???
          override def put(callbacks: BTreeRpc.PutCallbacks[Task]): Task[Unit] = {
            val childChecksums = Array("H<H<k1v1-cs>H<k2v2-cs>>".getBytes, "H<H<k4v4-cs>H<k5v5-cs>>".getBytes)
            for {
              _ ← callbacks.nextChildIndex(Array(key2.getBytes), childChecksums)
              _ ← callbacks.putDetails(Array(key4.getBytes, key5.getBytes), Array(val4Hash.getBytes, val5Hash.getBytes))
              _ ← callbacks.verifyChanges("H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k4v3-cs>H<k5v5-cs>>>".getBytes, wasSplitting = false)
              _ ← callbacks.changesStored()
            } yield ()
          }
        }

        val client = createClient(btreeRpc, "H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k4v4-cs>H<k5v5-cs>>>")
        val result = wait(client.put(key4, val3Hash.getBytes))
        result.get shouldBe val4Hash.getBytes // val4 is old value that was rewrited
      }

      // todo add case with tree rebalancing later
    }
  }

  private def wait[T](task: Task[T], time: FiniteDuration = 3.second)(implicit TS: TestScheduler): T = {
    val async = task.runAsync
    TS.tick(time)
    async.futureValue
  }

  private def createClient(bTreeRpc: BTreeRpc[Task], mRoot: String): MerkleBTreeClient[String] = {
    MerkleBTreeClient[String](
      Some(ClientState(mRoot.getBytes)),
      bTreeRpc,
      NoOpCrypt.forString,
      TestCryptoHasher
    )
  }

}
