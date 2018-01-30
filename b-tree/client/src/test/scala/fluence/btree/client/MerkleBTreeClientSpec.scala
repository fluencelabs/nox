package fluence.btree.client

import fluence.btree.client.MerkleBTreeClient.ClientState
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

  "getCmd" should {
    "returns error" when {

      "verifying server NextChildSearchResponse was failed" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createClient("H<H<k1v1-cs>>")
        val getCallbacks = wait(client.initGet(key1))
        val childChecksums = Array("H<H<k1v1>H<k2v2>>".getBytes, "H<H<k3v3>H<k4v4>>".getBytes)
        val result = wait(
          getCallbacks
            .nextChildIndex(Array("unexpected key returned from server".getBytes), childChecksums)
            .map(_ ⇒ ()).failed
        )

        result.getMessage should startWith("Checksum of branch didn't pass verifying")
      }

      "verifying server LeafResponse was failed" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createClient("H<H<k1v1>H<k2v2-cs>>")
        val getCallbacks = wait(client.initGet(key1))

        val result = wait(
          getCallbacks
            .submitLeaf(
              Array(key1.getBytes, "unexpected key returned from server".getBytes),
              Array(val1Hash.getBytes, val2Hash.getBytes)
            ).map(_ ⇒ ()).failed
        )

        result.getMessage should startWith("Checksum of leaf didn't pass verifying")
      }

    }

    "returns None" when {
      "key isn't found" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClient("H<H<k1v1-cs>>")
        val getCallbacks = wait(client.initGet(key2))

        val result = wait(getCallbacks.submitLeaf(Array(key1.getBytes), Array(val1Hash.getBytes)))

        result shouldBe None
      }
    }

    "returns founded result" when {
      "key was found in Root" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClient("H<H<k1v1-cs>>")
        val getCallbacks = wait(client.initGet(key1))

        val result = wait(getCallbacks.submitLeaf(Array(key1.getBytes), Array(val1Hash.getBytes)))

        result shouldBe Some(0)
      }

      "key was found at the second level of tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClient("H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k3v3-cs>H<k4v4-cs>>>")
        val getCallbacks = wait(client.initGet(key1))
        val childChecksums = Array("H<H<k1v1-cs>H<k2v2-cs>>".getBytes, "H<H<k3v3-cs>H<k4v4-cs>>".getBytes)

        val result = wait(
          for {
            _ ← getCallbacks.nextChildIndex(Array(key2.getBytes), childChecksums)
            idx ← getCallbacks.submitLeaf(Array(key1.getBytes, key2.getBytes), Array(val1Hash.getBytes, val2Hash.getBytes))
          } yield idx
        )

        result shouldBe Some(0)
      }
    }
  }

  "put" should {
    "returns error" when {
      "verifying server NextChildSearchResponse was failed" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createClient("H<H<k1v1>>")
        val putCallbacks = wait(client.initPut(key1, val1Hash.getBytes))

        val childChecksums = Array("H<H<k1v1>H<k2v2>>".getBytes, "H<H<k3v3>H<k4v4>>".getBytes)
        val result = wait(putCallbacks.nextChildIndex(Array("unexpected key returned from server".getBytes), childChecksums).failed)

        result.getMessage should startWith("Checksum of branch didn't pass verifying")
      }

      "verifying server LeafResponse was failed" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createClient("H<H<k1v1>H<k2v2>>")
        val putCallbacks = wait(client.initPut(key1, val1Hash.getBytes))

        val result = wait(putCallbacks.putDetails(
          Array(key1.getBytes, "unexpected key returned from server".getBytes),
          Array(val1Hash.getBytes, val2Hash.getBytes)
        ).failed)

        result.getMessage should startWith("Checksum of leaf didn't pass verifying")
      }
    }

    "put new key/value to tree" when {
      "key ins't present in tree (root inserting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClient("H<H<k1v1-cs>>")
        val putCallbacks = wait(client.initPut(key2, val2Hash.getBytes))

        wait(
          for {
            _ ← putCallbacks.putDetails(Array(key1.getBytes), Array(val1Hash.getBytes))
            _ ← putCallbacks.verifyChanges("H<H<k1v1-cs>H<k2v2-cs>>".getBytes, wasSplitting = false)
            _ ← putCallbacks.changesStored()
          } yield ()
        ) shouldBe ()
      }

      "key was found in tree (root inserting) " in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClient("H<H<k1v1-cs>>")
        val putCallbacks = wait(client.initPut(key1, val2Hash.getBytes))

        wait(
          for {
            _ ← putCallbacks.putDetails(Array(key1.getBytes), Array(val1Hash.getBytes))
            _ ← putCallbacks.verifyChanges("H<H<k1v2-cs>>".getBytes, wasSplitting = false)
            _ ← putCallbacks.changesStored()
          } yield ()
        ) shouldBe ()
      }

      "key ins't present in tree (second level inserting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClient("H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k4v4-cs>H<k5v5-cs>>>")
        val putCallbacks = wait(client.initPut(key3, val3Hash.getBytes))

        val childChecksums = Array("H<H<k1v1-cs>H<k2v2-cs>>".getBytes, "H<H<k4v4-cs>H<k5v5-cs>>".getBytes)
        val result = wait(
          for {
            _ ← putCallbacks.nextChildIndex(Array(key2.getBytes), childChecksums)
            _ ← putCallbacks.putDetails(Array(key4.getBytes, key5.getBytes), Array(val4Hash.getBytes, val5Hash.getBytes))
            _ ← putCallbacks.verifyChanges("H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k3v3-cs>H<k4v4-cs>H<k5v5-cs>>>".getBytes, wasSplitting = false)
            _ ← putCallbacks.changesStored()
          } yield ()
        ) shouldBe ()
      }

      "key was present in tree (second level inserting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createClient("H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k4v4-cs>H<k5v5-cs>>>")
        val putCallbacks = wait(client.initPut(key4, val3Hash.getBytes))
        val childChecksums = Array("H<H<k1v1-cs>H<k2v2-cs>>".getBytes, "H<H<k4v4-cs>H<k5v5-cs>>".getBytes)
        wait(
          for {
            _ ← putCallbacks.nextChildIndex(Array(key2.getBytes), childChecksums)
            _ ← putCallbacks.putDetails(Array(key4.getBytes, key5.getBytes), Array(val4Hash.getBytes, val5Hash.getBytes))
            _ ← putCallbacks.verifyChanges("H<H<k2>H<H<k1v1-cs>H<k2v2-cs>>H<H<k4v3-cs>H<k5v5-cs>>>".getBytes, wasSplitting = false)
            _ ← putCallbacks.changesStored()
          } yield ()
        ) shouldBe ()
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
      Some(ClientState(mRoot.getBytes)),
      NoOpCrypt.forString,
      TestCryptoHasher
    )
  }

}
