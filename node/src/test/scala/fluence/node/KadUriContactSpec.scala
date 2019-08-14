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

package fluence.node

import cats.data.EitherT
import cats.effect.{ContextShift, IO, Timer}
import cats.instances.either._
import cats.kernel.{Eq, Monoid}
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.codec.{CodecError, PureCodec}
import fluence.crypto.{Crypto, KeyPair}
import fluence.crypto.Crypto.liftCodecErrorToCrypto
import fluence.crypto.eddsa.Ed25519
import fluence.crypto.hash.CryptoHashers
import fluence.kad.KadRpcError
import fluence.kad.conf.AdvertizeConf
import fluence.kad.contact.{ContactAccess, UriContact}
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import fluence.kad.routing.LocalRouting
import fluence.kad.state.RoutingState
import fluence.log.{Log, LogFactory}
import fluence.node.workers.tendermint.TendermintNodeKey
import io.circe.parser._
import org.scalatest.{EitherValues, Matchers, WordSpec}
import cats.syntax.compose._
import cats.syntax.profunctor._
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext
import scala.concurrent.duration.Duration

class KadUriContactSpec extends WordSpec with EitherValues with Matchers {
  private val stage04Validator =
    """
      |{"priv_key":{"type":"tendermint/PrivKeyEd25519","value":"DWSj76VIufg0vgBjx1EDY4msDy2n/jOLCohj7tMoNO8shCCV/KcLSxeqHWvKzONTh6FE1UIW1ue+aDAprguAgg=="}}
    """.stripMargin

  private val keyPair =
    decode[TendermintNodeKey](stage04Validator)
      .flatMap(_.getKeyPair)
      .right
      .value

  // Lift Crypto errors for PureCodec errors
  private val sha256 = PureCodec.fromOtherFunc(
    CryptoHashers.Sha256
  )(err ⇒ CodecError("Crypto error when building Kademlia Key for Node[UriContact]", Some(err)))

  private val uriContactNodeCodec =
    new UriContact.NodeCodec(
      // We have tendermint's node_id in the Fluence Smart Contract now, which is first 20 bytes of sha256 of the public key
      // That's why we derive Kademlia key by sha1 of the node_id
      // sha1( sha256(p2p_key).take(20 bytes) )
      sha256
        .rmap(_.take(20))
        .lmap[KeyPair.Public](_.bytes) >>> Key.sha1
    )

  "contact" should {
    "generate and check (Tendermint keys)" in {
      val port = 25000.toShort
      val host = "207.154.210.117"
      val adv = AdvertizeConf(host, port)
      val expectedContact =
        "fluence://3zmnyo8cdqDDJn8pUgC7rv3mt7TVGaMVCLqdwyFv21Um:2ye9xNomz8a8ov4i8mv2FneSjSiYm7tMWniJPxTqii1LcDxkQ79osDk8vF9f6rPohb4DaDZoArmFBLiUeR4gwsMz@207.154.210.117:25000"

      val contact = (for {
        node <- uriContactNodeCodec.buildNode(adv, Ed25519.signAlgo.signer(keyPair))
        contactStr <- Crypto.fromOtherFunc(uriContactNodeCodec.writeNode).pointAt(node)
        _ <- UriContact.readAndCheckContact(Ed25519.signAlgo.checker).pointAt(contactStr)
      } yield contactStr).runF[Either[Throwable, ?]](())

      contact.left.map(e => println(s"Failed to check contact: $e ${e.printStackTrace()}"))

      contact.isRight shouldBe true
      contact.right.value shouldBe expectedContact
    }

    "update contacts" in {
      val node1 =
        uriContactNodeCodec.buildNode(AdvertizeConf("localhost", 25000), Ed25519.signAlgo.signer(keyPair)).unsafe(())
      val node2 =
        uriContactNodeCodec.buildNode(AdvertizeConf("127.0.0.1", 2500), Ed25519.signAlgo.signer(keyPair)).unsafe(())

      Eq[Key].eqv(node1.key, node2.key) shouldBe true

      implicit val cs: ContextShift[IO] = IO.contextShift(ExecutionContext.global)
      implicit val timer: Timer[IO] = IO.timer(ExecutionContext.global)
      implicit val log: Log[IO] = LogFactory.forPrintln[IO]().init("kad").unsafeRunSync()

      implicit val contactAccess: ContactAccess[IO, UriContact] = new ContactAccess[IO, UriContact](
        Duration.Inf,
        _ ⇒ IO(true),
        c ⇒
          new KademliaRpc[IO, UriContact] {

            /**
             * Ping the contact, get its actual Node status, or fail.
             */
            override def ping()(implicit log: Log[IO]): EitherT[IO, KadRpcError, Node[UriContact]] =
              EitherT.rightT(Node(Key.fromPublicKey.unsafe(c.signature.publicKey), c))

            /**
             * Perform a local lookup for a key, return K closest known nodes.
             *
             * @param key Key to lookup
             */
            override def lookup(key: Key, neighbors: Int)(
              implicit log: Log[IO]
            ): EitherT[IO, KadRpcError, Seq[Node[UriContact]]] = ???

            /**
             * Perform a local lookup for a key, return K closest known nodes, going away from the second key.
             *
             * @param key Key to lookup
             */
            override def lookupAway(key: Key, moveAwayFrom: Key, neighbors: Int)(
              implicit log: Log[IO]
            ): EitherT[IO, KadRpcError, Seq[Node[UriContact]]] = ???
        }
      )

      val rs = RoutingState.inMemory[IO, IO.Par, UriContact](Monoid[Key].empty, 3, 2).unsafeRunSync()
      val lr = LocalRouting(Monoid[Key].empty, rs.siblings, rs.bucket)

      lr.find(node1.key).unsafeRunSync() should be('empty)

      rs.update(node1).unsafeRunSync()
      lr.find(node1.key).unsafeRunSync() should be(Some(node1))

      rs.update(node2).unsafeRunSync()
      lr.find(node1.key).unsafeRunSync() should be(Some(node2))

    }

    "use correct tendermint node_id" in {
      val nodeId = ByteVector.fromValidHex("65c7af6d818fafcfec9d4b7b531a148f1ee6afd8")
      val pk =
        TendermintNodeKey(
          TendermintNodeKey.PrivKey(
            "",
            "DWSj76VIufg0vgBjx1EDY4msDy2n/jOLCohj7tMoNO8shCCV/KcLSxeqHWvKzONTh6FE1UIW1ue+aDAprguAgg=="
          )
        ).getKeyPair.right.get.publicKey

      uriContactNodeCodec.keyFromPublicKey.unsafe(pk) shouldBe Key.sha1.unsafe(nodeId.toArray)
    }
  }
}
