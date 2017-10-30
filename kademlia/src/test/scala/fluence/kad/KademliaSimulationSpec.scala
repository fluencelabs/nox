package fluence.kad

import java.nio.ByteBuffer
import java.time.Instant

import cats.Show
import cats.data.StateT
import cats.syntax.show._
import monix.eval.Coeval
import monix.execution.atomic.{ Atomic, AtomicBoolean }
import org.scalatest.{ Matchers, WordSpec }

import scala.collection.concurrent.TrieMap
import scala.concurrent.duration._
import scala.language.implicitConversions
import scala.util.Random

class KademliaSimulationSpec extends WordSpec with Matchers {
  implicit def key(i: Long): Key = Key(Array.concat(Array.ofDim[Byte](Key.Length - java.lang.Long.BYTES), {
    val buffer = ByteBuffer.allocate(java.lang.Long.BYTES)
    buffer.putLong(i)
    buffer.array()
  }))

  implicit def toLong(k: Key): Long =
    ByteBuffer.wrap(k.id.takeRight(java.lang.Long.BYTES)).getLong

  implicit val sk: Show[Key] = k ⇒ Console.CYAN + java.lang.Long.toBinaryString(k: Long).reverse.padTo(64, '-').reverse + Console.RESET
  implicit val sn: Show[Node[Long]] = n ⇒ s"Node(${n.key.show}, ${n.contact})"

  private val pingDuration = 1.second

  def bucketOps(maxBucketSize: Int): Bucket.WriteOps[Coeval, Long] =
    new Bucket.WriteOps[Coeval, Long] {
      private val buckets = TrieMap.empty[Int, Bucket[Long]]
      private val locks = TrieMap.empty[Int, Boolean].withDefaultValue(false)

      override protected def run[T](bucketId: Int, mod: StateT[Coeval, Bucket[Long], T]) = {
        require(!locks(bucketId), s"Bucket $bucketId must be not locked")
        locks(bucketId) = true
        println(s"Bucket $bucketId locked")

        mod.run(read(bucketId)).map {
          case (b, v) ⇒
            buckets(bucketId) = b
            v
        }.doOnFinish{ _ ⇒
          println(s"Bucket $bucketId unlocked")
          Coeval.now(locks.update(bucketId, false))
        }
      }

      override def read(bucketId: Int) =
        buckets.getOrElseUpdate(bucketId, Bucket(maxBucketSize))
    }

  def siblingsOps(nodeId: Key, maxSiblingsSize: Int): Siblings.WriteOps[Coeval, Long] =
    new Siblings.WriteOps[Coeval, Long] {
      private val state = Atomic(Siblings[Long](nodeId, maxSiblingsSize))
      private val lock = AtomicBoolean(false)

      override protected def run[T](mod: StateT[Coeval, Siblings[Long], T]) = {
        Coeval {
          require(lock.flip(true), "Siblings must be unlocked")
          lock.set(true)
        }.flatMap(_ ⇒
          mod.run(read).map {
            case (s, v) ⇒
              state.set(s)
              v
          }.doOnFinish(_ ⇒ Coeval.now(lock.flip(false))))
      }

      override def read =
        state.get
    }

  private def now = Instant.now()

  /**
   * In general, Kademlia network can't work correctly with a single thread, in blocking fashion.
   * It's possible that node A pings B, B pings A in return, A pings B and so on until stack overflows.
   */
  class KademliaCoeval(nodeId: Key, alpha: Int, k: Int, getKademlia: Long ⇒ Kademlia[Coeval, Long])(implicit BW: Bucket.WriteOps[Coeval, Long], SW: Siblings.WriteOps[Coeval, Long]) extends Kademlia[Coeval, Long](nodeId, alpha, pingDuration) {
    override def ownContact: Coeval[Node[Long]] = Coeval(Node[Long](nodeId, now, nodeId))

    override def rpc(contact: Long): KademliaRPC[Coeval, Long] = new KademliaRPC[Coeval, Long] {
      val kad = getKademlia(contact)

      /**
       * Ping the contact, get its actual Node status, or fail
       *
       * @return
       */
      override def ping() =
        kad.update(ownContact.value).flatMap(_ ⇒ kad.handleRPC.ping())

      /**
       * Perform a local lookup for a key, return K closest known nodes
       *
       * @param key Key to lookup
       * @return
       */
      override def lookup(key: Key, numberOfNodes: Int) =
        kad.update(ownContact.value).flatMap(_ ⇒ kad.handleRPC.lookup(key, numberOfNodes))

      /**
       * Perform an iterative lookup for a key, return K closest known nodes
       *
       * @param key Key to lookup
       * @return
       */
      override def lookupIterative(key: Key, numberOfNodes: Int) =
        kad.update(ownContact.value).flatMap(_ ⇒ kad.handleRPC.lookupIterative(key, numberOfNodes))

    }
  }

  "kademlia simulation" should {
    "launch with 200 nodes" in {
      // Kademlia's K
      val K = 16
      // Number of nodes in simulation
      val N = 200
      // Size of probe
      val P = 25

      val random = new Random(1000004)
      lazy val nodes: Map[Long, KademliaCoeval] =
        Stream.fill(N)(random.nextLong())
          .foldLeft(Map.empty[Long, KademliaCoeval]) {
            case (acc, n) ⇒
              acc + (n -> new KademliaCoeval(n, 3, K, nodes(_))(bucketOps(K), siblingsOps(n, K)))
          }

      val peers = nodes.keys.take(2).toSeq

      nodes.values.foreach(_.join(peers, K).run.onErrorHandle{
        e ⇒
          println(Console.RED + "Can't join within simulation" + Console.RESET)
          println(e)
          throw e
      }.value)

      //println("\n\n\n======================================\n\n\n")

      random.shuffle(nodes).take(P).foreach {
        case (i, ki) ⇒
          random.shuffle(nodes.values).take(P).foreach { kj ⇒

            val neighbors = kj.handleRPC.lookupIterative(i, K).value

            neighbors.size shouldBe (K min N)
            neighbors.map(_.contact) should contain(i)
          }
      }
    }
  }
}
