package fluence.kad

import java.nio.ByteBuffer

import org.scalatest.{Matchers, WordSpec}
import cats.instances.try_._

import scala.language.implicitConversions
import scala.util.{Random, Success, Try}

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

  class KademliaTry(nodeKey: Key, alpha: Int, k: Int, getKademlia: Long => Kademlia[Try, Long]) extends Kademlia[Try, Long] {
    private var state = RoutingTable[Long](nodeKey, k, k)
    private val nodeContact = Node[Long](nodeKey, nodeKey)

    override def rpc: (Long) => KademliaRPC[Try, Long] = l => {
      val krpc = getKademlia(l)
      new KademliaRPC[Try, Long] {
        override def ping(): Try[Node[Long]] =
          krpc.update(nodeContact).flatMap(_ => krpc.ping())

        override def lookup(key: Key): Try[Seq[Node[Long]]] =
          krpc.update(nodeContact).flatMap(_ => krpc.lookup(key))

        override def lookupIterative(key: Key): Try[Seq[Node[Long]]] =
          krpc.update(nodeContact).flatMap(_ => krpc.lookupIterative(key))
      }
    }

    override def ping(): Try[Node[Long]] = Success(nodeContact)

    override def lookup(key: Key): Try[Seq[Node[Long]]] =
      RoutingTable.lookup[Try, Long](key).run(state).map(_._2)

    override def lookupIterative(key: Key): Try[Seq[Node[Long]]] =
      RoutingTable.lookupIterative[Try, Long](key, k, alpha, rpc).run(state).map {
        case (rt, sq) =>
          state = rt
          sq
      }

    override def update(node: Node[Long]): Try[Unit] =
      RoutingTable.update(node, rpc).run(state).map {
        case (rt, _) =>
          state = rt
          ()
      }

    override def register(peers: Seq[Long]): Try[Unit] = {
      Try(peers.foreach { p =>
        rpc(p).lookupIterative(nodeKey).foreach(_.foreach { n =>
          rpc(n.contact).ping()
          update(n)
        })
      })
    }
  }

  "kademlia simulation" should {
    "launch with 100 nodes" in {
      val K = 32

      val random = new Random(1000004)
      lazy val nodes: Map[Long, Kademlia[Try, Long]] =
        Stream.fill(100)(random.nextLong())
          .foldLeft(Map.empty[Long, KademliaTry]) {
            case (acc, n) =>
              acc + (n -> new KademliaTry(n, 3, K, nodes(_)))
          }

      val peers = nodes.keys.take(2).toSeq

      nodes.values.foreach(_.register(peers))

      var z = 0

      random.shuffle(nodes).take(K/2).foreach { case (i, ki) =>
        println(Console.RED + z + Console.RESET)
        random.shuffle(nodes.values).take(K/2).foreach { kj =>
          print(".")
          val Success(neighbors) = kj.lookupIterative(i)

          neighbors.size shouldBe K
          neighbors.map(_.contact) should contain(i)
        }
        println()
        z = z + 1
      }
    }
  }
}
