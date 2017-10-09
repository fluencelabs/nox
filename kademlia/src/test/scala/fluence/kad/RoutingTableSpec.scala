package fluence.kad

import cats.Id
import cats.kernel.Monoid
import cats.instances.try_._
import org.scalatest.{Matchers, WordSpec}
import scala.language.implicitConversions

import scala.util.{Failure, Success, Try}

class RoutingTableSpec  extends WordSpec with Matchers{
  "kademlia routing table" should {
    implicit def key(i: Int): Key = Key(Array.concat(Array.ofDim[Byte](Key.Length - 1), Array(i.toByte)))

    "not fail when requesting its own key" in {
      val rt0 = RoutingTable(Monoid[Key].empty, 2)

      RoutingTable.find[Id](0).run(rt0)._2 should be('empty)
      RoutingTable.lookup[Id](0, 1).run(rt0)._2 should be('empty)
    }

    "finds nodes correctly" in {

      val rt0 = RoutingTable(Monoid[Key].empty, 2)

      val rt6 = (1 to 5).foldLeft(rt0){
        case (rt, i) =>
          val Success((rtU, _)) = RoutingTable.update[Try](Contact(i), _ => Failure(new NoSuchElementException)).run(rt)

          (1 to i).foreach {n =>
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
      val rt10 = (1 to 10).foldLeft(RoutingTable(Monoid[Key].empty, 2)){
        case (rtb, i) =>
          val Success((rtU, _)) = RoutingTable.update[Try](Contact(i), c => Success(c)).run(rtb)

          rtU
      }

      val (_, nbs10) = RoutingTable.lookup[Id](100, 10).run(rt10)
      nbs10.size shouldBe 7

      // Our implicit Int-to-Key conversion doesn't allow larger numbers
      val rt127 = (1 to 127).foldLeft(RoutingTable(Monoid[Key].empty, 2)){
        case (rtb, i) =>
          val Success((rtU, _)) = RoutingTable.update[Try](Contact(i), c => {
            Success(c)
          }).run(rtb)

          rtU
      }

      val (_, nbs127) = RoutingTable.lookup[Id](100, 10).run(rt127)
      nbs127.size shouldBe 10
    }
  }
}
