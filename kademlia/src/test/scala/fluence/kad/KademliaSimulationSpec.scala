package fluence.kad

import java.nio.ByteBuffer
import java.time.Instant

import cats.Show
import cats.data.StateT
import org.scalatest.{Matchers, WordSpec}
import cats.syntax.show._
import monix.eval.Coeval

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

  private def now = Instant.now()

  /**
    * In general, Kademlia network can't work correctly with a single thread, in blocking fashion.
    * It's possible that node A pings B, B pings A in return, A pings B and so on until stack overflows.
    *
    * @param nodeKey
    * @param alpha
    * @param k
    * @param getKademlia
    */
  class KademliaCoeval(nodeKey: Key, alpha: Int, k: Int, getKademlia: Long ⇒ Kademlia[Coeval, Long]) extends Kademlia[Coeval, Long](alpha, k, pingDuration) {
    private var state = RoutingTable[Long](nodeKey, k, k)

    def routingTable: RoutingTable[Long] = state

    override def ownContact: Node[Long] = Node[Long](nodeKey, now, nodeKey)

    override def rpc(contact: Long): KademliaRPC[Coeval, Long] = new KademliaRPC[Coeval, Long] {
      val kad = getKademlia(contact)

      /**
        * Ping the contact, get its actual Node status, or fail
        *
        * @return
        */
      override def ping() =
        kad.update(ownContact).flatMap(_ ⇒ kad.handleRPC.ping())

      /**
        * Perform a local lookup for a key, return K closest known nodes
        *
        * @param key Key to lookup
        * @return
        */
      override def lookup(key: Key, numberOfNodes: Int) =
        kad.update(ownContact).flatMap(_ ⇒ kad.handleRPC.lookup(key, numberOfNodes))

      /**
        * Perform an iterative lookup for a key, return K closest known nodes
        *
        * @param key Key to lookup
        * @return
        */
      override def lookupIterative(key: Key, numberOfNodes: Int) =
        kad.update(ownContact).flatMap(_ ⇒ kad.handleRPC.lookupIterative(key, numberOfNodes))

    }

    /**
      * Run some stateful operation, possibly mutating it
      *
      * @param mod Operation
      * @tparam T Return type
      * @return
      */
    override protected def run[T](mod: StateT[Coeval, RoutingTable[Long], T], l: String): Coeval[T] =
      mod.run(state).map {
        case (rt, v) ⇒
          state = rt
          v
      }.run

    /**
      * Non-blocking read request
      *
      * @param getter Getter for routing table
      * @tparam T Return type
      * @return
      */
    override protected def read[T](getter: RoutingTable[Long] => T): Coeval[T] =
      Coeval.now(getter(state))
  }

  "kademlia simulation" should {
    "launch with 500 nodes" in {
      // Kademlia's K
      val K = 16
      // Number of nodes in simulation
      val N = 500
      // Size of probe
      val P = 25

      val random = new Random(1000004)
      lazy val nodes: Map[Long, KademliaCoeval] =
        Stream.fill(N)(random.nextLong())
          .foldLeft(Map.empty[Long, KademliaCoeval]) {
            case (acc, n) ⇒
              acc + (n -> new KademliaCoeval(n, 3, K, nodes(_)))
          }

      val peers = nodes.keys.take(2).toSeq

      nodes.values.foreach(_.join(peers))

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
