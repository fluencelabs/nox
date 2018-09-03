package fluence.swarm
import cats.effect.IO
import com.softwaremill.sttp.SttpBackend
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.crypto.Crypto.Hasher
import fluence.swarm.ECDSASigner.Signer
import org.scalatest.{EitherValues, FlatSpec, Ignore, Matchers}
import org.web3j.crypto.{ECKeyPair, Keys}
import scodec.bits.ByteVector

import scala.util.Random

// TODO add more tests
/**
  * It works only when Swarm is working on local machine.
  */
@Ignore
class SwarmClientIntegrationSpec extends FlatSpec with Matchers with EitherValues {

  implicit val hasher: Hasher[ByteVector, ByteVector] = Keccak256Hasher.hasher

  val randomKeys: ECKeyPair = Keys.createEcKeyPair()
  val signer: Signer[ByteVector, ByteVector] = ECDSASigner.signer(randomKeys)

  private implicit val sttpBackend: SttpBackend[IO, Nothing] = AsyncHttpClientCatsBackend()

  val api = new SwarmClient[IO]("localhost", 8500)

  val ethAddress: ByteVector = ByteVector.fromHex(Keys.getAddress(randomKeys)).get

  "Error" should "be thrown if name is too big" in {
    val longName = Some("a good resource name" + "someotherwords" * 20)

    val frequency = 300L
    val time = 1528900000L
    val data = ByteVector.apply(1,2,3)

    val result = api.initializeMRU(longName, frequency, time, ethAddress, data, false, signer).value.unsafeRunSync()

    result.left.map(_.message).left.value should include("The name is too big")
  }

  "The correct result" must "be returned with an empty name" in {

    val name = None

    val frequency = 300L
    val time = 1528900000L
    val data = ByteVector(1,2,3)

    val result = api.initializeMRU(name, frequency, time, ethAddress, data, false, signer).value.unsafeRunSync()

    result should be ('right)
  }

  "The correct result" must "be returned with correct input" in {

    val name = Some("some name")

    val frequency = 300L
    val time = 1528900000L
    val data = ByteVector(1,2,3)

    val result = api.initializeMRU(name, frequency, time, ethAddress, data, false, signer).value.unsafeRunSync()

    result should be ('right)
  }

  "The client" can "download and upload data correctly" in {
    val data = ByteVector(1,2,3)
    val r = for {
      hash <- api.upload(data)
      result <- api.download(hash)
    } yield data.toArray shouldBe result
    r.value.unsafeRunSync() should be ('right)
  }

  "The client" can "upload, update and download mutable resources" in {

    val name = Some("some name")

    val frequency = 300L
    val time = System.currentTimeMillis() / 1000

    val rnd = Random

    val dataBytes1 = rnd.nextString(10).getBytes
    val data1 = ByteVector(dataBytes1)

    val dataBytes2 = rnd.nextString(12).getBytes
    val data2 = ByteVector(dataBytes2)

    val process = for {
      mruAddress <- api.initializeMRU(name, frequency, time, ethAddress, data1, false, signer)

      _ <- api.updateMRU(name, frequency, time, ethAddress, data2, false, 1, 2, signer)

      raw <- api.downloadRaw(mruAddress)
      _ = raw.entries.size should be (1)

      res1 <- api.downloadMRU(mruAddress, None)
      _ = res1 shouldBe dataBytes2

      res2 <- api.downloadMRU(mruAddress, Some(Period(1, Some(2))))
      _ = res2 shouldBe dataBytes2

      res3 <- api.downloadMRU(mruAddress, Some(Period(1, Some(1))))
      _ = res3 shouldBe dataBytes1

      meta <- api.downloadMRU(mruAddress, Some(Meta))
      _ = new String(meta) should (include(time.toString) and include(frequency.toString) and include(name.get))
    } yield {}

    process.value.unsafeRunSync() should be ('right)
  }
}
