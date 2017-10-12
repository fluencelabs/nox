package fluence.kad

import java.nio.ByteBuffer

import cats.data.StateT
import org.scalatest.{ Matchers, WordSpec }
import cats.instances.try_._

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

  class KademliaTry(nodeKey: Key, alpha: Int, k: Int, getKademlia: Long ⇒ Kademlia[Try, Long]) extends Kademlia[Try, Long](alpha, k) {
    private var state = RoutingTable[Long](nodeKey, k, k)

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
      val K = 32

      val random = new Random(1000004)
      lazy val nodes: Map[Long, Kademlia[Try, Long]] =
        Stream.fill(100)(random.nextLong())
          .foldLeft(Map.empty[Long, KademliaTry]) {
            case (acc, n) ⇒
              acc + (n -> new KademliaTry(n, 3, K, nodes(_)))
          }

      val peers = nodes.keys.take(2).toSeq

      nodes.values.foreach(_.join(peers))

      var z = 0

      random.shuffle(nodes).take(K / 2).foreach {
        case (i, ki) ⇒
          println(Console.RED + z + Console.RESET)
          random.shuffle(nodes.values).take(K / 2).foreach { kj ⇒
            print(".")
            val Success(neighbors) = kj.handleRPC().lookupIterative(i)

            neighbors.size shouldBe K
            neighbors.map(_.contact) should contain(i)
          }
          println()
          z = z + 1
      }
    }
  }
}
