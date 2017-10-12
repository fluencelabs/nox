package fluence.kad

import java.nio.ByteBuffer

import cats.Show
import cats.data.StateT
import org.scalatest.{ Matchers, WordSpec }
import cats.instances.try_._
import cats.syntax.show._

import scala.language.implicitConversions
import scala.util.{ Random, Success, Try }

class KademliaSimulationSpec extends WordSpec with Matchers {
  implicit def key(i: Long): Key = Key(Array.concat(Array.ofDim[Byte](Key.Length - java.lang.Long.BYTES), {
    val buffer = ByteBuffer.allocate(java.lang.Long.BYTES)
    buffer.putLong(i)
    buffer.array()
  }))

  implicit def toLong(k: Key): Long = {
    val buffer = ByteBuffer.allocate(java.lang.Long.BYTES)
    buffer.put(k.id.takeRight(java.lang.Long.BYTES))
    buffer.flip()
    buffer.getLong()
  }

  implicit val sk: Show[Key] = k ⇒ Console.CYAN + java.lang.Long.toBinaryString(k: Long).reverse.padTo(64, '-').reverse + Console.RESET
  implicit val sn: Show[Node[Long]] = n ⇒ s"Node(${n.key.show}, ${n.contact})"

  class KademliaTry(nodeKey: Key, alpha: Int, k: Int, getKademlia: Long ⇒ Kademlia[Try, Long]) extends Kademlia[Try, Long](alpha, k) {
    private var state = RoutingTable[Long](nodeKey, k, k)

    def routingTable: RoutingTable[Long] = state

    override def ownContact: Node[Long] = Node[Long](nodeKey, nodeKey)

    override def rpc(contact: Long): KademliaRPC[Try, Long] =
      getKademlia(contact).handleRPC(ownContact)

    override protected def run[T](mod: StateT[Try, RoutingTable[Long], T]): Try[T] =
      mod.run(state).map {
        case (rt, v) ⇒
          state = rt
          v
      }
  }

  "kademlia simulation" should {
    "launch with 100 nodes" in {
      val K = 16
      val N = 100
      val P = 25

      val random = new Random(1000004)
      lazy val nodes: Map[Long, KademliaTry] =
        Stream.fill(N)(random.nextLong())
          .foldLeft(Map.empty[Long, KademliaTry]) {
            case (acc, n) ⇒
              acc + (n -> new KademliaTry(n, 3, K, nodes(_)))
          }

      val peers = nodes.keys.take(2).toSeq

      nodes.values.foreach(_.join(peers))

      random.shuffle(nodes).take(P).foreach {
        case (i, ki) ⇒
          random.shuffle(nodes.values).take(P).foreach { kj ⇒
            val Success(neighbors) = kj.handleRPC().lookupIterative(i)

            neighbors.size shouldBe (K min N)
            neighbors.map(_.contact) should contain(i)
          }
      }
    }
  }
}
