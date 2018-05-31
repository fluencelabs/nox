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
  def createNewDataset(keysJson: Option[String], algo: SignAlgo, seed: Contact, keyPair: KeyPair)(
    implicit scheduler: Scheduler
  ): IO[ClientDatasetStorageApi[Task, Observable, String, String]] = {
    val algo: SignAlgo = Ecdsa.signAlgo
    import algo.checker

    val hasher: Crypto.Hasher[Array[Byte], Array[Byte]] = JsCryptoHasher.Sha256

    val kadConfig = KademliaConf(3, 3, 1, 5.seconds)

    val timeout = {
      val date = new Date(0)
      date.setSeconds(30)
      date
    }

    implicit val websocketMessageCodec: codec.PureCodec[WebsocketMessage, Array[Byte]] =
      PureCodec.build[WebsocketMessage, Array[Byte]](
        (m: WebsocketMessage) ⇒ m.toByteArray,
        (arr: Array[Byte]) ⇒ WebsocketMessage.parseFrom(arr)
      )

    val connectionPool = ConnectionPool[WebsocketMessage](timeout, 5.second, builder = Websocket.builder)
    val clientWebsocketServices = new ClientWebsocketServices(connectionPool)

    val client = clientWebsocketServices.build[Task]

    val clIO = FluenceClient.build(Seq(seed), algo, hasher, kadConfig, client andThen (_.get))

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
      (keyCrypt, valueCrypt) = cryptoMethods(keyPair.secretKey)
      dataset ← cl.createNewContract(keyPair, 2, keyCrypt, valueCrypt).toIO
    } yield dataset
  }
}
