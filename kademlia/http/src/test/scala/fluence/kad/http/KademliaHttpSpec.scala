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

package fluence.kad.http

import java.nio.file.Files

import cats.syntax.functor._
import cats.syntax.compose._
import cats.{Semigroup, Traverse}
import cats.data.Kleisli
import cats.effect.{ContextShift, IO, Resource, Timer}
import cats.instances.list._
import fluence.kad.protocol.Key
import fluence.log.{Log, LogFactory}
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector
import fluence.codec.PureCodec
import fluence.crypto.eddsa.Ed25519
import fluence.effects.kvstore.RocksDBStore
import fluence.effects.sttp.{SttpEffect, SttpStreamEffect}
import fluence.kad.conf.{AdvertizeConf, JoinConf, KademliaConfig, RoutingConf}
import fluence.kad.contact.UriContact
import fluence.kad.http.dht.DhtHttpNode
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto._
import org.http4s.{Response, Status}
import org.http4s.dsl.Http4sDsl
import org.http4s.server.Router
import org.http4s.server.blaze.BlazeServerBuilder

import scala.concurrent.ExecutionContext.global
import scala.concurrent.duration._
import scala.util.Random

class KademliaHttpSpec extends WordSpec with Matchers {
  implicit val shift: ContextShift[IO] = IO.contextShift(global)
  implicit val timer: Timer[IO] = IO.timer(global)

  implicit private val logFactory: LogFactory[IO] =
    LogFactory.forPrintln[IO](Log.Error)

  implicit private val log: Log[IO] =
    logFactory.init("KademliaHttpSpec").unsafeRunSync()

  private val sttpResource: Resource[IO, SttpStreamEffect[IO]] =
    SttpEffect.streamResource[IO]

  private val nodeCodec = new UriContact.NodeCodec(Key.fromPublicKey)

  "kademlia http transport" should {
    val signAlgo = Ed25519.signAlgo

    val tmpRoot = Files.createTempDirectory("kademliaHttpSpec")

    implicit val dsl: Http4sDsl[IO] = new Http4sDsl[IO] {}

    case class VersionedString(version: Int, data: String)

    implicit val stringCodec: PureCodec[Array[Byte], String] =
      PureCodec.build(new String(_), _.getBytes())
    implicit val versionedStringCodec: PureCodec[Array[Byte], VersionedString] =
      stringCodec >>> PureCodec.build[String, VersionedString](
        (s: String) ⇒ {
          val (v, d) = s.splitAt(s.indexOf(':'))
          VersionedString(v.toInt, d.drop(1))
        },
        (vs: VersionedString) ⇒ {
          val VersionedString(v, d) = vs
          s"$v:$d"
        }
      )
    implicit val vsEncoder: Encoder[VersionedString] = deriveEncoder
    implicit val vsDecoder: Decoder[VersionedString] = deriveDecoder

    implicit val versionedStringSemigroup: Semigroup[VersionedString] = {
      case (vs @ VersionedString(l, _), VersionedString(r, _)) if l >= r ⇒ vs
      case (_, vs) ⇒ vs
    }

    def node(port: Int, seeds: Seq[String])(implicit sttpBackend: SttpEffect[IO]) =
      for {
        n ← KademliaHttpNode
          .make[IO, IO.Par](
            KademliaConfig(
              RoutingConf(2, 2, 2, 2.seconds),
              AdvertizeConf("localhost", port.toShort),
              JoinConf(seeds, 2)
            ),
            signAlgo,
            signAlgo.generateKeyPair.unsafe(Some(ByteVector.fromInt(port).toArray)),
            tmpRoot.resolve(s"kad-$port"),
            nodeCodec
          )

        d ← DhtHttpNode
          .make[IO, VersionedString](
            "dht",
            RocksDBStore.makeRaw[IO](tmpRoot.resolve(s"dht-$port-data").toAbsolutePath.toString),
            RocksDBStore.makeRaw[IO](tmpRoot.resolve(s"dht-$port-meta").toAbsolutePath.toString),
            n.kademlia
          )

        _ ← BlazeServerBuilder[IO]
          .bindHttp(port, "localhost")
          .withHttpApp(
            Kleisli(
              a =>
                Router[IO](
                  "/kad" -> n.http.routes(),
                  "/dht" -> d.http.routes()
                ).run(a)
                  .getOrElse(
                    Response(Status.NotFound)
                      .withEntity(s"Route for ${a.method} ${a.pathInfo} ${a.params.mkString("&")} not found")
                  )
            )
          )
          .resource
      } yield (n, d)

    "connect client to server" in {
      (
        for {
          implicit0(sttpBackend: SttpEffect[IO]) ← sttpResource

          (node1, _) ← node(3210, Nil)

          (node2, _) ← node(3211, node1.kademlia.ownContact.unsafeRunSync().contact.toString :: Nil)

          _ ← Resource.liftF(node2.joinFiber.join)

        } yield (node1.kademlia, node2.kademlia)
      ).use {
        case (kad1, kad2) ⇒
          for {
            c1 ← kad1.ownContact
            c2 ← kad2.ownContact

            n2opt <- kad1.findNode(c2.key, 2)
            n1opt <- kad2.findNode(c1.key, 2)
          } yield {
            n1opt should be('nonEmpty)
            n2opt should be('nonEmpty)
          }
      }.unsafeRunSync()

    }

    "store and retrieve dht values" in {
      val startPort: Short = 3210

      (
        for {
          implicit0(sttpBackend: SttpEffect[IO]) ← sttpResource

          (node1, dht1) ← node(startPort, Nil)
          c1 ← Resource.liftF(node1.kademlia.ownContact.map(_.contact.toString))

          (node2, dht2) ← node(startPort + 1, c1 :: Nil)
          c2 ← Resource.liftF(node2.kademlia.ownContact.map(_.contact.toString))

          seeds ← Resource.liftF(node2.joinFiber.join).as(c1 :: c2 :: Nil)

          ns ← Traverse[List].traverse(List.range(startPort + 2, startPort + 20).map(_.toShort))(node(_, seeds))
          _ ← Traverse[List].traverse(ns.map(_._1.joinFiber))(f ⇒ Resource.liftF(f.join))

        } yield (dht1.dht, dht2.dht, ns.head._2.dht, ns.last._2.dht)
      ).use {
        case (d1, d2, d3, d4) ⇒
          val k1 = Key.fromStringSha1.unsafe("key one")
          val k2 = Key.fromStringSha1.unsafe("key two")
          val k3 = Key.fromStringSha1.unsafe("key three")
          val k4 = Key.fromStringSha1.unsafe("key four")

          val ds = d1 :: d2 :: d3 :: d4 :: Nil

          val r = new Random(124234123)

          def randomDht = r.shuffle(ds).head

          for {
            _ ← randomDht.put(k1, VersionedString(1, "test value")).value
            v1 ← randomDht.get(k1).value

            _ ← randomDht.put(k2, VersionedString(1, "test value2")).value
            v2 ← randomDht.get(k2).value

            _ ← randomDht.put(k3, VersionedString(1, "test value3")).value
            v3 ← randomDht.get(k3).value

            _ ← randomDht.put(k4, VersionedString(1, "test value4")).value
            v4 ← randomDht.get(k4).value

            _ ← randomDht.put(k4, VersionedString(2, "updated value4")).value
            v4bis ← randomDht.get(k4).value

            _ ← randomDht.put(k1, VersionedString(2, "updated value1")).value
            v1bis ← randomDht.get(k1).value

          } yield {
            v1 shouldBe Right(Some(VersionedString(1, "test value")))
            v2 shouldBe Right(Some(VersionedString(1, "test value2")))
            v3 shouldBe Right(Some(VersionedString(1, "test value3")))
            v4 shouldBe Right(Some(VersionedString(1, "test value4")))
            v4bis shouldBe Right(Some(VersionedString(2, "updated value4")))
            v1bis shouldBe Right(Some(VersionedString(2, "updated value1")))
          }
      }.unsafeRunSync()
    }
  }
}
