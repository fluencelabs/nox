package fluence.network.server

import java.time.Instant

import cats.data.StateT
import fluence.kad._
import fluence.network.Contact
import fluence.network.client.KademliaClient
import monix.eval.instances.ApplicativeStrategy
import monix.eval.{MVar, Task, TaskSemaphore}
import monix.execution.atomic.AtomicAny

import scala.collection.concurrent.TrieMap
import scala.concurrent.duration._
import scala.language.implicitConversions

class KademliaService(
    nodeId: Key,
    contact:          Task[Contact],
    client:           Contact ⇒ KademliaClient,
    k:                Int,
    parallelism:            Int            ,
    pingTimeout:      Duration
) extends Kademlia[Task, Contact](nodeId, parallelism, pingTimeout)(
  Task.catsInstances(ApplicativeStrategy.Parallel),
  KademliaService.bucketOps(k),
  KademliaService.siblingsOps(nodeId, k)
) {
  /**
   * Returns a network wrapper around a contact C, allowing querying it with Kademlia protocol
   *
   * @param contact Description on how to connect to remote node
   * @return
   */
  override def rpc(contact: Contact): KademliaRPC[Task, Contact] = client(contact)

  /**
   * How to promote this node to others
   *
   * @return
   */
  override def ownContact: Task[Node[Contact]] =
    contact.map(c ⇒ Node(nodeId, Instant.now(), c))

}

object KademliaService {
  def bucketOps(maxBucketSize: Int): Bucket.WriteOps[Task, Contact] =
    new Bucket.WriteOps[Task, Contact] {
      private val writeState = TrieMap.empty[Int, MVar[Bucket[Contact]]]
      private val readState = TrieMap.empty[Int, Bucket[Contact]]
      private val semaphore = TrieMap.empty[Int, TaskSemaphore]

      override protected def run[T](bucketId: Int, mod: StateT[Task, Bucket[Contact], T]): Task[T] =
        semaphore.getOrElseUpdate(bucketId, TaskSemaphore(1)).greenLight(
          for {
            bucket <- writeState.getOrElseUpdate(bucketId, MVar(read(bucketId))).read
            bv <- mod.run(bucket)
            _ <- writeState(bucketId).put(bv._1)
          } yield {
            readState(bucketId) = bv._1
            bv._2
          }
        )

      override def read(bucketId: Int): Bucket[Contact] =
        readState.getOrElseUpdate(bucketId, Bucket[Contact](maxBucketSize))
    }

  def siblingsOps(nodeId: Key, maxSiblings: Int): Siblings.WriteOps[Task, Contact] =
    new Siblings.WriteOps[Task, Contact] {
      private val readState = AtomicAny(Siblings[Contact](nodeId, maxSiblings))
      private val writeState = MVar(Siblings[Contact](nodeId, maxSiblings))
      private val semaphore = TaskSemaphore(1)

      override protected def run[T](mod: StateT[Task, Siblings[Contact], T]): Task[T] =
        semaphore.greenLight(
          for {
            siblings <- writeState.read
            sv <- mod.run(siblings)
            _ <- writeState.put(sv._1)
          } yield {
            readState.set(sv._1)
            sv._2
          }
        )

      override def read: Siblings[Contact] =
        readState.get
    }
}