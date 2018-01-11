package fluence.btree.server

import org.scalatest.{ Matchers, WordSpec }

class AssertionSpec extends WordSpec with Matchers {

  "assertions" should {
    "be enabled in tests" in {
      intercept[AssertionError] {
        Predef.assert(false, "Assertion error!")
        fail("Please enable assertions!")
      }
    }
  }
}
