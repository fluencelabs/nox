package fluence.kad

import cats.{Id, Show}
import cats.kernel.Monoid
import cats.instances.try_._
import cats.syntax.show._
import cats.syntax.monoid._
import cats.syntax.order._
import org.scalatest.{Matchers, WordSpec}

import scala.collection.immutable.SortedSet
import scala.language.implicitConversions
import scala.util.{Failure, Random, Success, Try}

class RoutingTableSpec extends WordSpec with Matchers {
  "kademlia routing table" should {
    implicit def key(i: Int): Key = Key(Array.concat(Array.ofDim[Byte](Key.Length - 1), Array(i.toByte)))

    implicit def toInt(k: Key): Int = k.id.last.toInt

    implicit val ks: Show[Key] = k => Console.MAGENTA + Integer.toBinaryString(k: Int).reverse.padTo(8, '+').reverse + Console.RESET + "/" + Console.YELLOW + (k: Int) + Console.RESET
    implicit val cs: Show[Contact] = c => s"Contact(${ks.show(c.key)})"

    "not fail when requesting its own key" in {
      val rt0 = RoutingTable(Monoid[Key].empty, 2)

      RoutingTable.find[Id](0).run(rt0)._2 should be('empty)
      RoutingTable.lookup[Id](0, 1).run(rt0)._2 should be('empty)
    }

    "finds nodes correctly" in {

      val rt0 = RoutingTable(Monoid[Key].empty, 2)

      val rt6 = (1 to 5).foldLeft(rt0) {
        case (rt, i) =>
          val Success((rtU, _)) = RoutingTable.update[Try](Contact(i), _ => Failure(new NoSuchElementException)).run(rt)

          (1 to i).foreach { n =>
            RoutingTable.find[Id](n).run(rtU)._2 should be('defined)
          }

          rtU
      }

      val Success((rt7, _)) = RoutingTable.update[Try](Contact(6), _ => Failure(new NoSuchElementException)).run(rt6)

      RoutingTable.find[Id](4).run(rt7)._2 should be('empty)

      val Success((rt8, _)) = RoutingTable.update[Try](Contact(6), c => Success(c)).run(rt6)

      RoutingTable.find[Id](4).run(rt8)._2 should be('defined)

    }

    "lookup nodes correctly" in {
      val rt10 = (1 to 10).foldLeft(RoutingTable(Monoid[Key].empty, 2)) {
        case (rtb, i) =>
          val Success((rtU, _)) = RoutingTable.update[Try](Contact(i), c => Success(c)).run(rtb)

          rtU
      }

      val (_, nbs10) = RoutingTable.lookup[Id](100, 10).run(rt10)
      nbs10.size shouldBe 7

      // Our implicit Int-to-Key conversion doesn't allow larger numbers
      val rt127 = (1 to 127).foldLeft(RoutingTable(Monoid[Key].empty, 10)) {
        case (rtb, i) =>
          val Success((rtU, _)) = RoutingTable.update[Try](Contact(i), c => {
            Success(c)
          }).run(rtb)

          rtU
      }

      (1 to 127).foreach { i =>
        val (_, nbs127) = RoutingTable.lookup[Id](i, 10).run(rt127)
        nbs127.size shouldBe 10
      }
    }

    "lookup nodes remotely" in {
      val nodes = collection.mutable.Map.empty[Int, RoutingTable]

      val random = new Random(253)

      def ping(c: Contact): Try[Contact] = {
        Success(c).filter(_ => random.nextBoolean())
      }

      def lookup(onNode: Int): (Key, Int) => Try[Seq[Contact]] =
        (k, n) => Try(nodes(onNode)).flatMap(RoutingTable.lookup[Try](k, n).run(_)).map(_._2)

      def lookupIterative(node: Int, on: Int, num: Int, log: Any => Unit = _ => ()): Try[Seq[Contact]] =
        Try(nodes(on)).flatMap { rt =>
          RoutingTable.lookupIterative[Try](node, num, 3, ping, (c, k, i) => lookup(c.key.id.last.toInt)(k, i), log)
            .run(rt).map {
            case (rt1, nds) =>
              nodes(on) = rt1
              nds
          }
        }

      def register(i: Int, on: Int) = {
        // register on known node
        lookupIterative(i, on, 20).foreach { cls0 =>
          // on's node is changed, save it
          nodes(on) =
            RoutingTable.update[Try](Contact(i), ping).run(nodes(on)).get._1

          // save found neighbors on i
          nodes(i) = (cls0 :+ Contact(on)).foldLeft(nodes(i)) {
            case (rt1, n) =>
              RoutingTable.update[Try](n, ping).run(rt1).get._1
          }
        }
      }

      (1 to 125).foreach(i => nodes(i) = RoutingTable(i, 10))

      (1 to 125).foreach { i =>
        register(i, 1)
        register(i, 57)
      }

      (1 to 125).foreach { i =>
        (1 to 125).filter(_ != i).filter(_ != 27).filter(_ != 28).filter(_ != 29).filter(_ != 30).filter(_ != 31).filter(_ != 42)
          .foreach { k =>
            (nodes(i).nodeId: Int) shouldBe i

            //println(Console.BLUE + "========================================================"+Console.RESET)

            //println((ss + (9: Key) + (8: Key) + (1: Key)).toVector.map(_.show))
            //println((ss + (9: Key) + (8: Key) + (1: Key) + (10: Key) + (100: Key)).toVector.map(_.show))

            val Success(neighbors) = lookupIterative(k, i, 10, if (k == 27) println else _ => ())
            //println(Console.YELLOW + "========================================================"+Console.RESET)


            val hasK = neighbors.map(_.key.id.last.toInt).contains(k)
            if (!hasK) {
              println("Looking for " + (k: Key).show + " in " + (i: Key).show)
              println("Prefix: " + ((k: Key) |+| (i: Key)).zerosPrefixLen)
              println("Found: " + neighbors.map(_.show))
              println("Local: " + RoutingTable.lookup(k, 10).run(nodes(i)).get._2.toVector.map(_.show))
              println(nodes(i).show)

              val knows = nodes.values.filter(rt =>
                RoutingTable.find[Id](k).run(rt)._2.isDefined
              ).map(_.nodeId)

              println("KNOWS: " + knows.map(kn => if (neighbors.exists(c => Key.OrderedKeys.compare(c.key, kn) == 0)) Console.RED + "!!!" + Console.RESET + kn.show else kn.show))

              val knows2 = nodes.values.filter(rt =>
                knows.exists(kn => Key.OrderedKeys.compare(kn, rt.nodeId) != 0 && RoutingTable.find[Id](kn).run(rt)._2.isDefined)
              ).map(_.nodeId)

              println("KNOWS2: " + knows2.map(kn => if (neighbors.exists(c => Key.OrderedKeys.compare(c.key, kn) == 0)) Console.RED + "!!!" + Console.RESET + kn.show else kn.show))

            }

            neighbors.size shouldBe 10
            hasK shouldBe true

          }
      }
    }
  }
}
