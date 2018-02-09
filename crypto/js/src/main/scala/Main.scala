import java.io.File
import java.math.BigInteger
import java.util.Base64

import fluence.crypto.algorithm.EcdsaJS
import fluence.crypto.facade.EC
import scodec.bits.ByteVector
import cats.instances.try_._
import fluence.crypto.{ FileKeyStorage, KeyStore }

import scala.concurrent.Promise
import scala.scalajs.js
import scala.util.Try
import js.JSConverters._
import scala.concurrent.ExecutionContext.Implicits.global

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

    import fluence.crypto.KeyStore._
    import io.circe.parser.decode
    import io.circe.syntax._
    import io.circe.{ Decoder, Encoder, HCursor, Json }

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
      |    "secret" : "JL6NmC4fkIrbTBw7nRE/MZThQB7Pwssl08N5uo6kbXc=",
      |    "public" : "A3fUOrIZWeZoAqJDipYEkuCIDm9sOe5hUY8/4Cep+ozi"
      |  }
      |}
    """.stripMargin

    val keyPair = decode[Option[KeyStore]](keyJson).right.get.get.keyPair
    val ecdsa = new EcdsaJS(ec)
    val msg = ByteVector(Array[Byte](1, 2, 3, 4, 5, 6))
    println("11111111")
    println("222222222")
    val sign = ecdsa.sign(keyPair, msg).get
    println("33333333333")
    val verifyRes = ecdsa.verify(sign, msg).get
    println("444444444")
    println(verifyRes)
  }
}

