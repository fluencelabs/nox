package fluence.client

import cats.effect.IO
import fluence.client.core.FluenceClient
import fluence.client.grpc.ClientWebsocketServices
import fluence.codec
import fluence.codec.PureCodec
import fluence.crypto.{Crypto, KeyPair}
import fluence.crypto.aes.{AesConfig, AesCrypt}
import fluence.crypto.ecdsa.Ecdsa
import fluence.crypto.hash.JsCryptoHasher
import fluence.crypto.signature.SignAlgo
import fluence.dataset.client.ClientDatasetStorageApi
import fluence.kad.KademliaConf
import fluence.kad.protocol.Contact
import fluence.proxy.grpc.WebsocketMessage
import fluence.transport.websocket.{ConnectionPool, Websocket}
import monix.eval.Task
import monix.execution.Scheduler
import monix.reactive.Observable

import scala.scalajs.js.Date
import scala.concurrent.duration._

object NaiveDataset {

  /**
   * Create always new fluence client, and return new dataset
   */
  def createNewDataset()(
    implicit scheduler: Scheduler
  ): IO[ClientDatasetStorageApi[Task, Observable, String, String]] = {
    val algo: SignAlgo = Ecdsa.signAlgo
    import algo.checker

    val hasher: Crypto.Hasher[Array[Byte], Array[Byte]] = JsCryptoHasher.Sha256

    val seedContact = Contact.readB64seed.unsafe(
      "eyJwayI6IkE5ZmZaWS1FbG5aSlNCWEJBMno4Q2FpWTNLT051Y3doTkdfY0FmRVNNU3liIiwicHYiOjB9.eyJhIjoiMTI3LjAuMC4xIiwiZ3AiOjExMDIxLCJnaCI6IjAwMDAwMDAwMDAwMDAwMDAwMDAwIiwid3AiOjgwOTF9.MEUCIAu0lDokN_cMOZzgVXzCdPNPhhFVWEBkhP5vbv_EGUL3AiEA73MbbvNAANW6BTin-jho9Dsv42X2iqtgv-s5vpgGdQo="
    )

    val kadConfig = KademliaConf(3, 3, 1, 5.seconds)

    val timeout = {
      val date = new Date(0)
      date.setSeconds(3)
      date
    }

    implicit val websocketMessageCodec: codec.PureCodec[WebsocketMessage, Array[Byte]] =
      PureCodec.build[WebsocketMessage, Array[Byte]](
        (m: WebsocketMessage) ⇒ m.toByteArray,
        (arr: Array[Byte]) ⇒ WebsocketMessage.parseFrom(arr)
      )

    val connectionPool = ConnectionPool[WebsocketMessage](timeout, 1.second, builder = Websocket.builder)
    val clientWebsocketServices = new ClientWebsocketServices(connectionPool)

    val client = clientWebsocketServices.build[Task]

    val clIO = FluenceClient.build(Seq(seedContact), algo, hasher, kadConfig, client andThen (_.get))

    def cryptoMethods(
      secretKey: KeyPair.Secret
    ): (Crypto.Cipher[String], Crypto.Cipher[String]) = {
      val aesConfig = AesConfig()
      (
        AesCrypt.forString(secretKey.value, withIV = false, aesConfig),
        AesCrypt.forString(secretKey.value, withIV = false, aesConfig)
      )
    }

    for {
      cl ← clIO
      newkey ← algo.generateKeyPair.runF[IO](None)
      (keyCrypt, valueCrypt) = cryptoMethods(newkey.secretKey)
      dataset ← cl.createNewContract(newkey, 2, keyCrypt, valueCrypt).toIO
    } yield dataset
  }
}
