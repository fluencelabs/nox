package crypto.fluence

import org.scalatest.WordSpec

import scalajs.js
import js.Dynamic.{ global ⇒ g }
import js.DynamicImplicits._
import scalajs.js.annotation

import scala.concurrent.Promise
import scala.concurrent.ExecutionContext.Implicits.global

class SomeSpec extends WordSpec {

  val cr = g.require("crypto")

  "Some" should {
    "work" in {

    }
  }
}
