package fluence.swarm
import cats.effect.IO
import com.softwaremill.sttp.SttpBackend
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.crypto.Crypto.Hasher
import fluence.swarm.crypto.Secp256k1Signer.Signer
import fluence.swarm.crypto.{Keccak256Hasher, Secp256k1Signer}
import org.scalatest.{EitherValues, FlatSpec, Matchers}
import org.web3j.crypto.{ECKeyPair, Keys}
import scodec.bits.ByteVector
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.duration._
import scala.language.postfixOps
import scala.util.Random

// TODO add more tests
/**
 * It works only when Swarm is working on local machine.
 */
//@Ignore
class SwarmClientIntegrationSpec extends FlatSpec with Matchers with EitherValues {

  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.INFO

  implicit val hasher: Hasher[ByteVector, ByteVector] = Keccak256Hasher.hasher

  val randomKeys: ECKeyPair = Keys.createEcKeyPair()
  val signer: Signer[ByteVector, ByteVector] = Secp256k1Signer.signer(randomKeys)

  private implicit val sttpBackend: SttpBackend[IO, Nothing] = AsyncHttpClientCatsBackend()

  val api = new SwarmClient[IO]("localhost", 8500)

  val ethAddress: ByteVector = ByteVector.fromHex(Keys.getAddress(randomKeys)).get

  "Error" should "be thrown if name is too big" in {
    val longName = Some("a good resource name" + "someotherwords" * 20)

    val frequency = 300 seconds
    val time = 1528900000 seconds
    val data = ByteVector.apply(1, 2, 3)

    val result =
      api.initializeMutableResource(longName, frequency, time, ethAddress, data, false, signer).value.unsafeRunSync()

    result.left.map(_.message).left.value should include("The name is too big")
  }

  "The correct result" must "be returned with an empty name" in {

    val name = None

    val frequency = 300 seconds
    val time = 1528900000L seconds
    val data = ByteVector(1, 2, 3)

    val result =
      api.initializeMutableResource(name, frequency, time, ethAddress, data, false, signer).value.unsafeRunSync()

    result should be('right)
  }

  "The correct result" must "be returned with correct input" in {

    val name = Some("some name")

    val frequency = 300 seconds
    val time = 1528900000L seconds
    val data = ByteVector(1, 2, 3)

    val result =
      api.initializeMutableResource(name, frequency, time, ethAddress, data, false, signer).value.unsafeRunSync()

    result should be('right)
  }

  "The client" can "download and upload data correctly" in {
    val data = ByteVector(1, 2, 3)
    val r = for {
      hash <- api.upload(data)
      result <- api.download(hash)
    } yield data.toArray shouldBe result
    r.value.unsafeRunSync() should be('right)
  }

  "The client" can "upload, update and download mutable resources" in {

    val name = Some("some name")

    val frequency = 300 seconds
    val time = System.currentTimeMillis() millis

    val rnd = Random

    val dataBytes1 = rnd.nextString(10).getBytes
    val data1 = ByteVector(dataBytes1)

    val dataBytes2 = rnd.nextString(12).getBytes
    val data2 = ByteVector(dataBytes2)

    val process = for {
      mruAddress <- api.initializeMutableResource(name, frequency, time, ethAddress, data1, false, signer)

      _ <- api.updateMutableResource(name, frequency, time, ethAddress, data2, false, 1, 2, signer)

      mruManifest <- api.downloadRaw(mruAddress)
      _ = mruManifest.entries.size should be(1)

      latest <- api.downloadMutableResource(mruAddress, None)
      _ = latest shouldBe data2

      version2 <- api.downloadMutableResource(mruAddress, Some(Period(1, Some(2))))
      _ = version2 shouldBe data2

      version1 <- api.downloadMutableResource(mruAddress, Some(Period(1, Some(1))))
      _ = version1 shouldBe data1

      meta <- api.downloadMutableResource(mruAddress, Some(Meta))
      _ = new String(meta.toArray) should (include(time.toSeconds.toString) and include(frequency.toSeconds.toString) and include(
        name.get
      ))
    } yield {}

    val res = process.value.unsafeRunSync()
    println("RES === " + res)
    res should be('right)
  }
}
