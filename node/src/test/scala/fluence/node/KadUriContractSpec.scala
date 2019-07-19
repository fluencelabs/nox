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
import fluence.crypto.Crypto
import fluence.crypto.Crypto.liftCodecErrorToCrypto
import fluence.crypto.eddsa.Ed25519
import fluence.kad.KadRpcError
import fluence.kad.http.UriContact
import fluence.kad.protocol.{ContactAccess, KademliaRpc, Key, Node}
import fluence.kad.routing.{LocalRouting, RoutingTable}
import fluence.kad.state.{RoutingState, SiblingsState}
import fluence.log.{Log, LogFactory}
import fluence.node.workers.tendermint.TendermintPrivateKey
import io.circe.parser._
import org.scalatest.{EitherValues, Matchers, WordSpec}

import scala.concurrent.ExecutionContext
import scala.concurrent.duration.Duration

class KadUriContractSpec extends WordSpec with EitherValues with Matchers {
  private val stage04Validator =
    """
      |{
      |  "address": "9F16F63227F11942E6E4A3282B2A293E4BF8206C",
      |  "pub_key": {
      |    "type": "tendermint/PubKeyEd25519",
      |    "value": "vAs+M0nQVqntR6jjPqTsHpJ4bsswA3ohx05yorqveyc="
      |  },
      |  "priv_key": {
      |    "type": "tendermint/PrivKeyEd25519",
      |    "value": "okWxDfeg+uHCT5qpPoUhbBxJL7yOH/+zsPok6VK9OLy8Cz4zSdBWqe1HqOM+pOweknhuyzADeiHHTnKiuq97Jw=="
      |  }
      |}
    """.stripMargin

  private val keyPair = {
    val key = decode[TendermintPrivateKey](stage04Validator).right.value
    TendermintPrivateKey.getKeyPair(key).right.value
  }

  "contact" should {
    "generate and check (Tendermint keys)" in {
      val port = 25000.toShort
      val host = "207.154.210.117"
      val expectedContact =
        "fluence://Df3bFWKN6tb2ejyPKfUceA57i6RwMvLfoi5NA3QZ3aSi:4zGZc3BSeyWsEiB6BuHN7gBheu1uAQn3cFpTTYZy6L43v9wUj9qgMuWtAAVg5LNV8B8xxLqPagVFU39YsbrpQQhT@207.154.210.117:25000"

      val contact = (for {
        node <- UriContact.buildNode(host, port, Ed25519.signAlgo.signer(keyPair))
        contactStr <- Crypto.fromOtherFunc(UriContact.writeNode).pointAt(node)
        _ <- UriContact.readAndCheckContact(Ed25519.signAlgo.checker).pointAt(contactStr)
      } yield contactStr).runF[Either[Throwable, ?]](())

      contact.left.map(e => println(s"Failed to check contact: $e ${e.printStackTrace()}"))

      contact.isRight shouldBe true
      contact.right.value shouldBe expectedContact
    }

    "update contacts" in {
      val node1 = UriContact.buildNode("localhost", 25000.toShort, Ed25519.signAlgo.signer(keyPair)).unsafe(())
      val node2 = UriContact.buildNode("127.0.0.1", 2500.toShort, Ed25519.signAlgo.signer(keyPair)).unsafe(())

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
      lr.find(node1.key).unsafeRunSync() should be(node1)

      rs.update(node2).unsafeRunSync()
      lr.find(node1.key).unsafeRunSync() should be(node2)

    }
  }
}
