package fluence.swarm
import java.math.BigInteger

import cats.effect.IO
import com.softwaremill.sttp.asynchttpclient.AsyncHttpClientBackend
import com.softwaremill.sttp.{HttpURLConnectionBackend, SttpBackend}
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.swarm.requests.InitializeMutableResourceRequest
import org.scalatest.{FlatSpec, Matchers}
import org.web3j.crypto.{ECKeyPair, Keys, Sign}
import scodec.bits.ByteVector

// TODO add more tests
class SomeAnotherSpec extends FlatSpec with Matchers {

  implicit val hasher = Keccak256Hasher.hasher

  val randomKeys = Keys.createEcKeyPair()
  val signer = ECDSASigner.signer(randomKeys)

  private implicit val sttpBackend: SttpBackend[IO, Nothing] = AsyncHttpClientCatsBackend()

  val api = new SwarmClient[IO]("localhost", 8500)

  "Error" should "be thrown if name is too big" in {
    val longName = Some("a good resource name" + "someotherwords" * 20)
    //  println("NAME LENGTH === " + name.get.size)
    val name = Option.empty[String]
    val frequency = 300L
    //  val time = System.currentTimeMillis() / 1000
    val time = 1528900000L

    val data = ByteVector.apply(1,2,3)

    val ethAddress = ByteVector.fromHex(Keys.getAddress(randomKeys)).get

    val result = api.initializeMRU(longName, frequency, time, ethAddress, data, false, signer).value.unsafeRunSync()

    println(result)

  }
}
