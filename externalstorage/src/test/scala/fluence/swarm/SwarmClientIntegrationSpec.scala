/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.swarm
import cats.effect.IO
import com.softwaremill.sttp.SttpBackend
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.swarm.crypto.Secp256k1Signer.Signer
import fluence.swarm.crypto.Secp256k1Signer
import org.scalatest.{EitherValues, FlatSpec, Ignore, Matchers}
import org.web3j.crypto.{ECKeyPair, Keys}
import scodec.bits.ByteVector

import scala.concurrent.duration._
import scala.language.postfixOps
import scala.util.Random

// TODO add more tests
/**
 * It works only when Swarm is working on local machine.
 */
@Ignore
class SwarmClientIntegrationSpec extends FlatSpec with Matchers with EitherValues {

  val randomKeys: ECKeyPair = Keys.createEcKeyPair()
  val signer: Signer[ByteVector, ByteVector] = Secp256k1Signer.signer(randomKeys)

  private implicit val sttpBackend: SttpBackend[IO, Nothing] = AsyncHttpClientCatsBackend()

  val api = SwarmClient("localhost", 8500)

  val ethAddress: ByteVector = ByteVector.fromHex(Keys.getAddress(randomKeys)).get

  "Error" should "be thrown if name is too big" in {
    val longName = Some("a good resource name" + "someotherwords" * 20)

    val frequency = 300 seconds
    val time = 1528900000 seconds
    val id = MutableResourceIdentifier(longName, frequency, time, ethAddress)
    val data = ByteVector.apply(1, 2, 3)

    val result =
      api.initializeMutableResource(id, data, false, signer).value.unsafeRunSync()

    result.left.map(_.message).left.value should include("The name is too big")
  }

  "The correct result" must "be returned with an empty name" in {

    val name = None

    val frequency = 300 seconds
    val time = 1528900000L seconds
    val id = MutableResourceIdentifier(name, frequency, time, ethAddress)
    val data = ByteVector(1, 2, 3)

    val result =
      api.initializeMutableResource(id, data, false, signer).value.unsafeRunSync()

    result should be('right)
  }

  "The correct result" must "be returned with correct input" in {

    val name = Some("some name")

    val frequency = 300 seconds
    val time = 1528900000L seconds
    val id = MutableResourceIdentifier(name, frequency, time, ethAddress)
    val data = ByteVector(1, 2, 3)

    val result =
      api.initializeMutableResource(id, data, false, signer).value.unsafeRunSync()

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

    val id = MutableResourceIdentifier(name, frequency, time, ethAddress)

    val process = for {
      mruAddress <- api.initializeMutableResource(id, data1, false, signer)

      _ <- api.updateMutableResource(id, data2, false, 1, 2, signer)

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

    res should be('right)
  }
}
