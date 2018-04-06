/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.kad

import java.nio.ByteBuffer
import java.time.Instant

import cats.syntax.show._
import cats.Show
import cats.data.EitherT
import fluence.kad.protocol.{Key, Node}
import fluence.kad.testkit.TestKademlia
import monix.eval.Coeval
import monix.execution.atomic.AtomicInt
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.duration._
import scala.language.implicitConversions
import scala.util.Random

class KademliaSimulationSpec extends WordSpec with Matchers {
  implicit def key(i: Long): Key =
    Key.fromBytes.unsafe(Array.concat(Array.ofDim[Byte](Key.Length - java.lang.Long.BYTES), {
      ByteVector.fromLong(i).toArray
    }))

  def keyToLong(k: Key): Long =
    ByteBuffer.wrap(k.id.takeRight(java.lang.Long.BYTES)).getLong

  implicit val sk: Show[Key] =
    (k: Key) ⇒ Console.CYAN + java.lang.Long.toBinaryString(keyToLong(k)).reverse.padTo(64, '-').reverse + Console.RESET

  implicit val sn: Show[Node[Long]] =
    (n: Node[Long]) ⇒ s"Node(${n.key.show}, ${n.contact})"

  private val pingDuration = 1.second

  private def now = Instant.now()

  //TODO ~10 minute duration is very long for regular testing
  "kademlia simulation" ignore {
    "launch with 200 nodes" in {
      // Kademlia's K
      val K = 16
      // Number of nodes in simulation
      val N = 200
      // Size of probe
      val P = 25

      val random = new Random(1000004)
      lazy val nodes: Map[Long, Kademlia[Coeval, Long]] =
        Stream
          .fill(N)(random.nextLong())
          .foldLeft(Map.empty[Long, Kademlia[Coeval, Long]]) {
            case (acc, n) ⇒
              acc + (n -> TestKademlia.coeval(n, 3, K, nodes(_), keyToLong, pingDuration))
          }

      val peers = nodes.keys.take(2).toSeq

      nodes.values.foreach(_.join(peers, K).run.onErrorHandle { e ⇒
        println(Console.RED + "Can't join within simulation" + Console.RESET)
        println(e)
        throw e
      }.value)

      //println("\n\n\n======================================\n\n\n")

      random.shuffle(nodes).take(P).foreach {
        case (i, ki) ⇒
          random.shuffle(nodes.values).take(P).filterNot(_.nodeId === ki.nodeId).foreach { kj ⇒
            val neighbors = kj.lookupIterative(i, K).value

            neighbors.size shouldBe (K min N)
            neighbors.map(_.contact) should contain(i)
          }
      }
    }

    // pre-building a network the same way as it's done above
    // Kademlia's K
    val K = 16
    // Number of nodes in simulation
    val N = 400
    // Size of probe
    val P = 25

    val random = new Random(1000004)

    val nodes = TestKademlia.coevalSimulation[Long](K, N, keyToLong, random.nextLong(), joinPeers = 2)

    "callIterative: make no more requests then limit in callIterative" in {
      val kad = nodes.drop(N / 4).head._2

      val counter = AtomicInt(0)

      kad
        .callIterative[Throwable, Unit](
          nodes.last._1,
          _ ⇒ EitherT(Coeval(counter.increment()).map[Either[Throwable, Unit]](_ ⇒ Left(new NoSuchElementException()))),
          K min P,
          K max P max (N / 3)
        )
        .value shouldBe empty

      counter.get shouldBe (K max P max (N / 3))
    }

    "callIterative: make no less requests then num+parallelism in idempotent callIterative" in {
      val kad = nodes.drop(N / 3).head._2

      val counter = AtomicInt(0)
      val numToFind = K min P

      kad
        .callIterative[Throwable, Unit](
          nodes.last._1,
          _ ⇒ EitherT.rightT[Coeval, Throwable](counter.increment()),
          K min P,
          K max P max (N / 3),
          isIdempotentFn = true
        )
        .value
        .size shouldBe counter.get

      counter.get should be <= (numToFind + 3)
      counter.get should be >= numToFind
    }

    "callIterative: make num calls in non-idempotent callIterative" in {
      val kad = nodes.drop(N / 3).head._2

      val counter = AtomicInt(0)
      val numToFind = K min P

      kad
        .callIterative[Throwable, Unit](
          nodes.last._1,
          _ ⇒ EitherT.rightT[Coeval, Throwable](counter.increment()),
          K min P,
          K max P max (N / 3),
          isIdempotentFn = false
        )
        .value
        .size shouldBe numToFind

      counter.get shouldBe numToFind
    }
  }
}
