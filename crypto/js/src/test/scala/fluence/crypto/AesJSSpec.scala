package fluence.crypto

import org.scalatest.{Matchers, WordSpec}

import scala.util.Random

class AesSpec extends WordSpec with Matchers with slogging.LazyLogging {

  def rndString(size: Int): String = Random.nextString(10)

  "aes crypto" should {
    "work with IV" in {

    }

    "work without IV" in {

    }
  }
}
