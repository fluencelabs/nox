package fluence.swarm
import java.math.BigInteger

import fluence.swarm.requests.InitializeMutableResourceRequest
import org.scalatest.{FlatSpec, Matchers}
import org.web3j.crypto.{ECKeyPair, Keys, Sign}
import scodec.bits.ByteVector

// TODO add more tests
class SomeSpec extends FlatSpec with Matchers {

  implicit val hasher = Keccak256Hasher.hasher

  "Metadata" should "be correct" in {
    val name = "a good resource name"
    val frequency = 300L
    val time = 1528900000

    val secret = new BigInteger(1, ByteVector.fromHex("facadefacadefacadefacadefacadefacadefacadefacadefacadefacadefaca").get.toArray)
    val publicKey = Sign.publicKeyFromPrivate(secret)
    val someKP = new ECKeyPair(secret, publicKey)
    val signer = ECDSASigner.signer(someKP)

    val ethAddress = ByteVector.fromValidHex(Keys.getAddress(someKP))

    val data = ByteVector("This hour's update: Swarm 99.0 has been released!".getBytes)

    val checkedRootAddr = "0x6e744a730f7ea0881528576f0354b6268b98e35a6981ef703153ff1b8d32bbef"
    val checkedMetaHash = "0x0c0d5c18b89da503af92302a1a64fab6acb60f78e288eb9c3d541655cd359b60"
    val checkedData = "0x5468697320686f75722773207570646174653a20537761726d2039392e3020686173206265656e2072656c656173656421"
    val checkedSign = "0x8adc0dc4dd464f874da5f524ed0a2ebac02185fed3e862cc130d3514ffb570f470abebbbb4ec3d96397fc46c5f87def63f56db7b4199e51a9caabda4ef6899f100"

    val req = InitializeMutableResourceRequest(Some(name), frequency, time, ethAddress, data, false, signer).value.right.get

    "0x" + req.data.toHex shouldBe checkedData
    "0x" + req.metaHash.hash.toHex shouldBe checkedMetaHash
    "0x" + req.rootAddr.addr.toHex shouldBe checkedRootAddr
    "0x" + req.signature.signature.toHex shouldBe checkedSign
  }
}
