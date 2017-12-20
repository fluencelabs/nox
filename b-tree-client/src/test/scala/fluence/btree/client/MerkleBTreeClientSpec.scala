package fluence.btree.client

import fluence.btree.client.core.ClientState
import fluence.btree.client.network._
import fluence.crypto.NoOpCrypt
import fluence.hash.TestCryptoHasher
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

  val val1 = "v1"
  val val2 = "v2"
  val val3 = "v3"
  val val4 = "v4"
  val val5 = "v5"

  "get" should {
    "returns error" when {
      "network returns error" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val error = new IllegalArgumentException("Broken pipe")
        val network = new BTreeClientNetwork[Task, String, String] {
          override def doRequest(state: RequestState): Task[(RequestState, BTreeServerResponse)] = {
            Task.raiseError(error) // server returns error
          }
        }
        val client = createClient(network, "H<H<k1>>")
        val exception = wait(client.get(key1).failed)
        exception.getMessage shouldBe error.getMessage
      }

      "verifying server NextChildSearchResponse was failed" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val network = new BTreeClientNetwork[Task, String, String] {
          override def doRequest(state: RequestState): Task[(RequestState, BTreeServerResponse)] = {
            state match {
              case GetState(_, _, _, InitGetRequest) ⇒ // server returns branch node with invalid keys for searching
                val childChecksums = Array("H<H<k1v1>H<k2v2>>".getBytes, "H<H<k3v3>H<k4v4>>".getBytes)
                Task(state → NextChildSearchResponse(Array("unexpected key returned from server".getBytes), childChecksums))
              case _ ⇒ ??? // not needed for this test case
            }
          }
        }

        val client = createClient(network, "H<H<k1v1>>")
        val exception = wait(client.get(key1).failed)
        exception.getMessage should startWith("Checksum of branch didn't pass verifying")
      }

      "verifying server LeafResponse was failed" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val network = new BTreeClientNetwork[Task, String, String] {
          override def doRequest(state: RequestState): Task[(RequestState, BTreeServerResponse)] = {
            state match {
              case GetState(_, _, _, InitGetRequest) ⇒ // server returns leaf node with invalid keys for searching
                Task(state, LeafResponse(
                  Array(key1.getBytes, "unexpected key returned from server".getBytes),
                  Array(val1.getBytes, val2.getBytes)
                ))
            }
          }
        }
        val client = createClient(network, "H<H<k1v1>H<k2v2>>")
        val exception = wait(client.get(key1).failed)
        exception.getMessage should startWith("Checksum of leaf didn't pass verifying")
      }

    }

    "returns None" when {
      "key isn't found" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val network = new BTreeClientNetwork[Task, String, String] {
          override def doRequest(state: RequestState): Task[(RequestState, BTreeServerResponse)] = {
            state match {
              case GetState(_, _, _, InitGetRequest) ⇒ // server returns root-leaf with one key/value
                Task(state, LeafResponse(Array(key1.getBytes), Array(val1.getBytes)))
            }
          }
        }
        val client = createClient(network, "H<H<k1v1>>")
        val result = wait(client.get(key2))
        result shouldBe None
      }
    }

    "returns founded result" when {
      "key was found in Root" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val network = new BTreeClientNetwork[Task, String, String] {
          override def doRequest(state: RequestState): Task[(RequestState, BTreeServerResponse)] = {
            state match {
              case GetState(_, _, _, InitGetRequest) ⇒ // server returns root-leaf with one key/value
                Task(state → LeafResponse(Array(key1.getBytes), Array(val1.getBytes)))
            }
          }
        }
        val client = createClient(network, "H<H<k1v1>>")
        val result = wait(client.get(key1))
        result.get shouldBe val1
      }

      "key was found at the second level of tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val network = new BTreeClientNetwork[Task, String, String] {
          override def doRequest(state: RequestState): Task[(RequestState, BTreeServerResponse)] = {
            state match {
              case GetState(_, _, _, InitGetRequest) ⇒ // server returns root-branch with one key and two children
                val childChecksums = Array("H<H<k1v1>H<k2v2>>".getBytes, "H<H<k3v3>H<k4v4>>".getBytes)
                Task(state → NextChildSearchResponse(Array(key2.getBytes), childChecksums))
              case GetState(_, _, _, ResumeSearchRequest(0)) ⇒ // server returns root-leaf with two key/value
                Task(state, LeafResponse(Array(key1.getBytes, key2.getBytes), Array(val1.getBytes, val2.getBytes)))
            }
          }
        }

        val client = createClient(network, "H<H<k2>H<H<k1v1>H<k2v2>>H<H<k3v3>H<k4v4>>>")
        val result = wait(client.get(key1))
        result.get shouldBe val1
      }
    }
  }

  "put" should {
    "returns error" when {
      "network returns error" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val error = new IllegalArgumentException("Broken pipe")
        val network = new BTreeClientNetwork[Task, String, String] {
          override def doRequest(state: RequestState): Task[(RequestState, BTreeServerResponse)] = {
            Task.raiseError(error) // server returns error
          }
        }
        val client = createClient(network, "H<H<k1>>")
        val exception = wait(client.put(key1, val1).failed)
        exception.getMessage shouldBe error.getMessage
      }

      "verifying server NextChildSearchResponse was failed" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val network = new BTreeClientNetwork[Task, String, String] {
          override def doRequest(state: RequestState): Task[(RequestState, BTreeServerResponse)] = {
            state match {
              case PutState(_, _, _, _, InitPutRequest, _) ⇒ // server returns root with invalid keys for searching
                val childChecksums = Array("H<H<k1v1>H<k2v2>>".getBytes, "H<H<k3v3>H<k4v4>>".getBytes)
                Task(state → NextChildSearchResponse(Array("unexpected key returned from server".getBytes), childChecksums))
              case _ ⇒ ??? // not needed for this test case
            }
          }
        }

        val client = createClient(network, "H<H<k1v1>>")
        val exception = wait(client.put(key1, val1).failed)
        exception.getMessage should startWith("Checksum of branch didn't pass verifying")
      }

      "verifying server LeafResponse was failed" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val network = new BTreeClientNetwork[Task, String, String] {
          override def doRequest(state: RequestState): Task[(RequestState, BTreeServerResponse)] = {
            state match {
              case PutState(_, _, _, _, InitPutRequest, _) ⇒ // server returns leaf with invalid keys for searching
                Task(state, LeafResponse(
                  Array(key1.getBytes, "unexpected key returned from server".getBytes),
                  Array(val1.getBytes, val2.getBytes)
                ))
            }
          }
        }
        val client = createClient(network, "H<H<k1v1>H<k2v2>>")
        val exception = wait(client.put(key1, val1).failed)
        exception.getMessage should startWith("Checksum of leaf didn't pass verifying")
      }

    }

    "put new key/value to tree" when {
      "key ins't present in tree (root inserting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val network = new BTreeClientNetwork[Task, String, String] {
          override def doRequest(state: RequestState): Task[(RequestState, BTreeServerResponse)] = {
            state match {
              case PutState(_, _, _, _, InitPutRequest, _) ⇒ // server returns root-leaf with one key/value
                Task(state, LeafResponse(Array(key1.getBytes), Array(val1.getBytes)))
              case PutState(_, _, _, _, PutRequest(_, _, _), _) ⇒ // server inserts new value and asks confirmation
                Task(state, VerifySimplePutResponse("H<H<k1v1>H<k2v2>>".getBytes))
              case PutState(_, _, _, _, Confirm, _) ⇒ // server accepts confirmation
                Task(state, ConfirmAccepted)
            }
          }
        }
        val client = createClient(network, "H<H<k1v1>>")
        val result = wait(client.put(key2, val2))
        result shouldBe None
      }

      "key was found in tree (root inserting) " in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val network = new BTreeClientNetwork[Task, String, String] {
          override def doRequest(state: RequestState): Task[(RequestState, BTreeServerResponse)] = {
            state match {
              case PutState(_, _, _, _, InitPutRequest, _) ⇒ // server returns root-branch with one key and two children
                Task(state, LeafResponse(Array(key1.getBytes), Array(val1.getBytes)))
              case PutState(_, _, _, _, PutRequest(_, _, _), _) ⇒ // server rewrite old value with the new value and asks confirmation
                Task(state, VerifySimplePutResponse("H<H<k1v2>>".getBytes))
              case PutState(_, _, _, _, Confirm, _) ⇒ // server accepts confirmation
                Task(state, ConfirmAccepted)
            }
          }
        }

        val client = createClient(network, "H<H<k1v1>>")
        val result = wait(client.put(key1, val2))
        result.get shouldBe val1 // val1 is old value that was rewrited
      }

      "key ins't present in tree (second level inserting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val network = new BTreeClientNetwork[Task, String, String] {
          override def doRequest(state: RequestState): Task[(RequestState, BTreeServerResponse)] = {
            state match {
              case PutState(_, _, _, _, InitPutRequest, _) ⇒ // server returns root with key2 for define next child
                val childChecksums = Array("H<H<k1v1>H<k2v2>>".getBytes, "H<H<k4v4>H<k5v5>>".getBytes)
                Task(state → NextChildSearchResponse(Array(key2.getBytes), childChecksums))
              case PutState(_, _, _, _, ResumeSearchRequest(1), _) ⇒ // server returns founded leaf to client
                Task(state, LeafResponse(Array(key4.getBytes, key5.getBytes), Array(val4.getBytes, val5.getBytes)))
              case PutState(_, _, _, _, PutRequest(_, _, _), _) ⇒ // server inserts new value and asks confirmation
                Task(state, VerifySimplePutResponse("H<H<k2>H<H<k1v1>H<k2v2>>H<H<k3v3>H<k4v4>H<k5v5>>>".getBytes))
              case PutState(_, _, _, _, Confirm, _) ⇒ // server accepts confirmation
                Task(state, ConfirmAccepted)
            }
          }
        }

        val client = createClient(network, "H<H<k2>H<H<k1v1>H<k2v2>>H<H<k4v4>H<k5v5>>>")
        val result = wait(client.put(key3, val3))
        result shouldBe None
      }

      "key was present in tree (second level inserting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val network = new BTreeClientNetwork[Task, String, String] {
          override def doRequest(state: RequestState): Task[(RequestState, BTreeServerResponse)] = {
            state match {
              case PutState(_, _, _, _, InitPutRequest, _) ⇒ // server returns root with key2 for define next child
                val childChecksums = Array("H<H<k1v1>H<k2v2>>".getBytes, "H<H<k4v4>H<k5v5>>".getBytes)
                Task(state → NextChildSearchResponse(Array(key2.getBytes), childChecksums))
              case PutState(_, _, _, _, ResumeSearchRequest(1), _) ⇒ // server returns founded leaf to client
                Task(state, LeafResponse(Array(key4.getBytes, key5.getBytes), Array(val4.getBytes, val5.getBytes)))
              case PutState(_, _, _, _, PutRequest(_, _, _), _) ⇒ // server inserts new value and asks confirmation
                Task(state, VerifySimplePutResponse("H<H<k2>H<H<k1v1>H<k2v2>>H<H<k4v3>H<k5v5>>>".getBytes))
              case PutState(_, _, _, _, Confirm, _) ⇒ // server accepts confirmation
                Task(state, ConfirmAccepted)
            }
          }
        }

        val client = createClient(network, "H<H<k2>H<H<k1v1>H<k2v2>>H<H<k4v4>H<k5v5>>>")
        val result = wait(client.put(key4, val3))
        result.get shouldBe val4 // val4 is old value that was rewrited
      }

      // todo add case with tree rebalancing later
    }
  }

  private def wait[T](task: Task[T], time: FiniteDuration = 3.second)(implicit TS: TestScheduler): T = {
    val async = task.runAsync
    TS.tick(time)
    async.futureValue
  }

  private def createClient(network: BTreeClientNetwork[Task, String, String], mRoot: String) = {
    MerkleBTreeClient[Task, String, String](
      ClientState(mRoot.getBytes),
      network,
      NoOpCrypt.forString,
      NoOpCrypt.forString,
      TestCryptoHasher
    )
  }

}
