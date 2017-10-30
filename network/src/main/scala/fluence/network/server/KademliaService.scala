package fluence.network.server

import java.time.Instant

import cats.data.StateT
import fluence.kad._
import monix.eval.instances.ApplicativeStrategy
import monix.eval.{ MVar, Task }
import monix.execution.atomic.AtomicAny

import scala.collection.concurrent.TrieMap
import scala.concurrent.duration._
import scala.language.implicitConversions

/**
 * Kademlia service to be launched as a singleton on local node
 * @param nodeId Current node ID
 * @param contact Node's contact to advertise
 * @param client Getter for RPC calling of another nodes
 * @param maxSiblingsSize Maximum number of siblings to store, e.g. K * Alpha
 * @param maxBucketSize Maximum size of a bucket, usually K
 * @param parallelism Parallelism factor (named Alpha in paper)
 * @param pingExpiresIn Duration to avoid too frequent ping requests, used in [[Bucket.update()]]
 * @tparam C Contact info
 */
class KademliaService[C](
    nodeId:          Key,
    contact:         Task[C],
    client:          C ⇒ KademliaRPC[Task, C],
    maxSiblingsSize: Int,
    maxBucketSize:   Int,
    parallelism:     Int,
    pingExpiresIn:   Duration
) extends Kademlia[Task, C](nodeId, parallelism, pingExpiresIn)(
  Task.catsInstances(ApplicativeStrategy.Parallel),
  KademliaService.bucketOps(maxBucketSize),
  KademliaService.siblingsOps(nodeId, maxSiblingsSize)
) {
  /**
   * Returns a network wrapper around a contact C, allowing querying it with Kademlia protocol
   *
   * @param contact Description on how to connect to remote node
   * @return
   */
  override def rpc(contact: C): KademliaRPC[Task, C] = client(contact)

  /**
   * How to promote this node to others
   *
   * @return
   */
  override def ownContact: Task[Node[C]] =
    contact.map(c ⇒ Node(nodeId, Instant.now(), c))

}

object KademliaService {
  /**
   * Performs atomic update on a MVar, blocking asynchronously if another update is in progress
   *
   * @param mvar State variable
   * @param mod Modifier
   * @param updateRead Callback to update read model
   * @tparam S State
   * @tparam T Return value
   */
  private def runOnMVar[S, T](mvar: MVar[S], mod: StateT[Task, S, T], updateRead: S ⇒ Unit): Task[T] =
    mvar.take.flatMap { init ⇒
      // Run modification
      mod.run(init).onErrorHandleWith { err ⇒
        // In case modification failed, write initial value back to MVar
        mvar.put(init).flatMap(_ ⇒ Task.raiseError(err))
      }
    }.flatMap {
      case (updated, value) ⇒
        // Update read and write states
        updateRead(updated)
        mvar.put(updated).map(_ ⇒ value)
    }

  /**
   * Builds asynchronous bucket ops with $maxBucketSize nodes in each bucket
   * @param maxBucketSize Max number of nodes in each bucket
   * @tparam C Node contacts type
   */
  def bucketOps[C](maxBucketSize: Int): Bucket.WriteOps[Task, C] =
    new Bucket.WriteOps[Task, C] {
      private val writeState = TrieMap.empty[Int, MVar[Bucket[C]]]
      private val readState = TrieMap.empty[Int, Bucket[C]]

      override protected def run[T](bucketId: Int, mod: StateT[Task, Bucket[C], T]): Task[T] =
        runOnMVar(
          writeState.getOrElseUpdate(bucketId, MVar(read(bucketId))),
          mod,
          readState.update(bucketId, _: Bucket[C])
        )

      override def read(bucketId: Int): Bucket[C] =
        readState.getOrElseUpdate(bucketId, Bucket[C](maxBucketSize))
    }

  /**
   * Builds asynchronous sibling ops with $maxSiblings nodes max
   * @param nodeId Siblings are sorted by distance to this nodeId
   * @param maxSiblings Max number of closest siblings to store
   * @tparam C Node contacts type
   */
  def siblingsOps[C](nodeId: Key, maxSiblings: Int): Siblings.WriteOps[Task, C] =
    new Siblings.WriteOps[Task, C] {
      private val readState = AtomicAny(Siblings[C](nodeId, maxSiblings))
      private val writeState = MVar(Siblings[C](nodeId, maxSiblings))

      override protected def run[T](mod: StateT[Task, Siblings[C], T]): Task[T] =
        runOnMVar(
          writeState,
          mod,
          readState.set
        )

      override def read: Siblings[C] =
        readState.get
    }
}