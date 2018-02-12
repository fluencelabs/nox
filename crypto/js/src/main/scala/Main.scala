import java.util.Base64

import cats.instances.try_._
import fluence.crypto.KeyStore
import fluence.crypto.algorithm.EcdsaJS
import fluence.crypto.facade.{EC, SHA256}
import fluence.crypto.signature.Signature
import scodec.bits.ByteVector

import scala.scalajs.js
import scala.scalajs.js.JSConverters._

object Main extends js.JSApp {
  def main(): Unit = {
    println("Hello world!")

    val ec = new EC("secp256k1")
    println(ec)
    val seedLit = js.Dynamic.literal(entropy = 123123)
    val key = ec.genKeyPair(Some(seedLit))

    println("111111111")

    try {
      val priv = key.getPrivate("hex")
      println(priv)

      println("base64 = " + Base64.getEncoder.encode(priv.getBytes()))
      println("bg = " + priv)
    } catch {
      case e: Throwable ⇒
        println(e.getLocalizedMessage)
        e.printStackTrace()
    }

    try {
      val publ = key.getPublic(true, "hex")
      println(publ)
      println(publ)
      val publV = ByteVector(publ.getBytes()).toBase64
      println("base64 = " + publV)

      """
        |{
        |  "keystore" : {
        |    "secret" : "cJPTCysRAodOwwJ4+NmpkiqeykqlN/JR+/nJ4m7srNs=",
        |    "public" : "A+XOJmqI+stpbztgyOL40/oT0ks3ucn/WDpc4YsKDBfc"
        |  }
      """.stripMargin
    } catch {
      case e: Throwable ⇒ println(e.getLocalizedMessage)
    }

    println("22222222")

    try {
      val javaPriv = ByteVector.fromBase64("cJPTCysRAodOwwJ4+NmpkiqeykqlN/JR+/nJ4m7srNs=").get.toHex
      val javaPub = ByteVector.fromBase64("A+XOJmqI+stpbztgyOL40/oT0ks3ucn/WDpc4YsKDBfc").get.toHex

      val options = js.Dynamic.literal(pub = javaPub, pubEnc = "hex", priv = javaPriv, privEnc = "hex")
      val jkp = ec.keyPair(options)
      println(jkp)
      println(jkp.getPrivate("hex"))
      println(jkp.getPublic(true, "hex"))

      val msg = js.Array[Byte](1, 2, 3, 4, 5, 6)
      val sign = jkp.sign(msg)
      println(sign)
      val der = sign.toDER("hex")
      println("der == " + der)
      //      println("der == " + der.toArray)

      println("CHECK === " + jkp.verify(msg, der))
    } catch {
      case e: Throwable ⇒ println(e.getLocalizedMessage)
    }

    try {
      val msg = js.Array[Byte](1, 2, 3, 4, 5, 6)
      val sign = key.sign(msg)

      println("sign as is === " + sign)
      val der = sign.toDER("hex")
      println("der == " + der)
      val publV = ByteVector.fromHex(der)
    } catch {
      case e: Throwable ⇒ e.printStackTrace()
    }

    import io.circe.parser.decode
    import io.circe.syntax._

    println("=================== ECDSA TESTING =======================")
    try {
      val ecdsa = new EcdsaJS(ec)
      val msg = ByteVector(Array[Byte](1, 2, 3, 4, 5, 6))
      println("11111111")
      val keys = ecdsa.generateKeyPair().get
      println("222222222")
      val sign = ecdsa.sign(keys, msg).get
      println("33333333333")
      val verifyRes = ecdsa.verify(sign, msg).get
      println("444444444")
      println(verifyRes)

      val json = KeyStore(keys).asJson
      println(json.spaces2)
    } catch {
      case e: Throwable ⇒
        println(e.getLocalizedMessage)
        e.printStackTrace()
    }

    val keyJson = """
      |{
      |  "keystore" : {
      |    "secret" : "lb+4PQHqXuRZjGrLHqj/zHvBWlpOCijzMime1FKkoW4=",
      |    "public" : "A2h7CZpapgCmsh+ZkKWdkXli7t4N5/LfoyUeglOmvXkl"
      |  }
      |}
    """.stripMargin

    val sign1hex = "3044022057ca7253d5a5ddb83b2b068cbd81c34f6e7765d3e16bfd5de9de55571b2ce6440220633d3d5c0cbbfd34afe6459f1ba5b38be8d58163afc0e622a3f03fa18256ab3b"

    val keyPair = decode[Option[KeyStore]](keyJson).right.get.get.keyPair
    val ecdsa = new EcdsaJS(ec)
    val msg = ByteVector(Array[Byte](1, 2, 3, 4, 5, 6))
    val data = ByteVector(Array[Byte](1, 2, 3, 4, 5, 10))
    println("11111111")
    println("222222222")
    val sign = ecdsa.sign(keyPair, msg).get

    println("33333333333")

    println("JAVASCRIPT DATA === " + data.toArray.mkString(","))
    val sha256 = new SHA256()
    sha256.update(data.toArray.toJSArray)

    val dig = sha256.digest("hex")
    val shadata = ByteVector.fromHex(dig).get
    println("JAVASCRIPT SHA256 === " + shadata.toArray.mkString(","))
    println("JAVASCRIPT ARR SHA256 === " + dig.toJSArray)

    println("aaaaaaaa")

    val sign1 = Signature(keyPair.publicKey, ByteVector.fromHex(sign1hex).get)

    println("bbbbbbbbbb")

    val verifyRes = ecdsa.verify(sign, msg).get

    println("ccccccccccc")
    val verifyRes3 = ecdsa.verify(sign.copy(publicKey = keyPair.publicKey), msg).get
    println("ddddddddddd")
    val verifyRes1 = ecdsa.verify(sign1, shadata).get
    println("444444444")
    println(verifyRes)
    println("HEYHEY PUBLIKKEY ========== " + verifyRes3)
    println("HEYHEY ========== " + verifyRes1)

  }
}

