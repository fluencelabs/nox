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

package fluence.kad.testkit

import cats.effect.{ConcurrentEffect, ContextShift, IO, LiftIO, Timer}
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.applicative._
import cats.Parallel
import cats.data.EitherT
import cats.instances.stream._
import fluence.crypto.KeyPair
import fluence.crypto.signature.Signer
import fluence.kad.conf.RoutingConf
import fluence.kad.contact.ContactAccess
import fluence.kad.{KadRpcError, Kademlia}
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import fluence.kad.routing.RoutingTable
import fluence.log.Log

import scala.concurrent.duration._
import scala.language.higherKinds

object TestKademlia {

  def apply[F[_]: Parallel: Timer: ConcurrentEffect: LiftIO, P[_], C](
    nodeId: Key,
    alpha: Int,
    k: Int,
    getKademlia: C ⇒ Kademlia[F, C],
    toContact: Key ⇒ C,
    pingExpiresIn: FiniteDuration = 1.second
  ): F[Kademlia[F, C]] = {
    def ownContactValue = Node[C](nodeId, toContact(nodeId))

    implicit val ca = new ContactAccess[F, C](
      pingExpiresIn,
      _ ⇒ true.pure[F],
      contact ⇒
        new KademliaRpc[F, C] {
          private val kad = getKademlia(contact)

          private def updateOwn(implicit log: Log[F]): EitherT[F, KadRpcError, Boolean] =
            EitherT.liftF(kad.update(ownContactValue))

          /**
           * Ping the contact, get its actual Node status, or fail
           */
          override def ping()(implicit log: Log[F]) =
            updateOwn >> kad.handleRPC.ping()

          /**
           * Perform a local lookup for a key, return K closest known nodes
           *
           * @param key Key to lookup
           */
          override def lookup(key: Key, numberOfNodes: Int)(implicit log: Log[F]) =
            updateOwn >> kad.handleRPC.lookup(key, numberOfNodes)

          /**
           * Perform a local lookup for a key, return K closest known nodes, going away from the second key
           *
           * @param key Key to lookup
           */
          override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int)(implicit log: Log[F]) =
            updateOwn >> kad.handleRPC.lookupAway(key, moveAwayFrom, numberOfNodes)
      }
    )

    RoutingTable[F, P, C](nodeId, k, k)
      .map(Kademlia[F, P, C](_, ownContactValue.pure[F], RoutingConf(k, k, alpha, pingExpiresIn)))

  }

  def simulationIO[C](
    k: Int,
    n: Int,
    toContact: Key ⇒ C,
    nextRandomKey: ⇒ Key,
    joinPeers: Int = 0,
    alpha: Int = 3,
    pingExpiresIn: FiniteDuration = 1.second
  )(implicit t: Timer[IO], cs: ContextShift[IO], log: Log[IO]): Map[C, Kademlia[IO, C]] = {
    lazy val kads: Map[C, Kademlia[IO, C]] =
      Parallel
        .parTraverse(
          Stream
            .fill(n)(nextRandomKey)
        )(
          rk ⇒
            log.scope("key" -> toContact(rk).toString)(
              implicit log ⇒ apply(rk, alpha, k, kads(_), toContact, pingExpiresIn)
          )
        )
        .unsafeRunSync()
        .foldLeft(Map.empty[C, Kademlia[IO, C]]) {
          case (acc, kad) ⇒
            acc + (toContact(kad.nodeKey) -> kad)
        }

    val peers = kads.keys.take(joinPeers).toSeq

    if (peers.nonEmpty)
      kads.values.foreach(
        kd ⇒ log.scope("key" -> toContact(kd.nodeKey).toString)(implicit log ⇒ kd.join(peers, k)).unsafeRunSync()
      )

    kads
  }

  def simulationKP[C](
    k: Int,
    n: Int,
    toContact: Key ⇒ C,
    nextRandomKeyPair: ⇒ KeyPair,
    joinPeers: Int = 0,
    alpha: Int = 3,
    pingExpiresIn: FiniteDuration = 1.second
  )(implicit t: Timer[IO], cs: ContextShift[IO], log: Log[IO]): Map[C, (Signer, Kademlia[IO, C])] = {
    import fluence.crypto.DumbCrypto.signAlgo

    lazy val kads: Map[C, (Signer, Kademlia[IO, C])] =
      Parallel
        .parTraverse[Stream, IO, IO.Par, KeyPair, (C, (Signer, Kademlia[IO, C]))](
          Stream.fill(n)(nextRandomKeyPair)
        ) { keyPair ⇒
          val signer = signAlgo.signer(keyPair)
          val key = Key.fromPublicKey.unsafe(keyPair.publicKey)
          apply[IO, IO.Par, C](key, alpha, k, kads(_)._2, toContact, pingExpiresIn).map(
            kad ⇒ toContact(key) -> (signer, kad)
          )
        }
        .unsafeRunSync()
        .toMap

    val peers = kads.keys.take(joinPeers).toSeq

    if (peers.nonEmpty)
      kads.values.foreach(_._2.join(peers, k).unsafeRunSync())

    kads
  }
}
