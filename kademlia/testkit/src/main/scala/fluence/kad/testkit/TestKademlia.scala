/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.kad.testkit

import java.time.Instant

import cats.effect.{IO, LiftIO}
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.{~>, Applicative, Monad, MonadError, Parallel}
import fluence.crypto.SignAlgo
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signer
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import fluence.kad.{Bucket, Kademlia, Siblings}
import monix.eval.Coeval

import scala.concurrent.duration._
import scala.language.higherKinds

object TestKademlia {
  implicit val CoevalParallel: Parallel[Coeval, Coeval] = new Parallel[Coeval, Coeval] {
    override def applicative = Applicative[Coeval]

    override def monad = Monad[Coeval]

    override def sequential: Coeval ~> Coeval = new (Coeval ~> Coeval) {
      override def apply[A](fa: Coeval[A]) = fa
    }

    override def parallel = sequential
  }

  implicit val liftCoeval: LiftIO[Coeval] = new LiftIO[Coeval] {
    override def liftIO[A](ioa: IO[A]): Coeval[A] = Coeval(ioa.unsafeRunSync())
  }

  def apply[F[_]: Monad: LiftIO, G[_], C](
    nodeId: Key,
    alpha: Int,
    k: Int,
    getKademlia: C ⇒ Kademlia[F, C],
    toContact: Key ⇒ C,
    toIO: F ~> IO,
    pingExpiresIn: FiniteDuration = 1.second
  )(
    implicit
    BW: Bucket.WriteOps[F, C],
    SW: Siblings.WriteOps[F, C],
    P: Parallel[F, G]
  ): Kademlia[F, C] = {
    def ownContactValue = Node[C](nodeId, Instant.now(), toContact(nodeId))
    Kademlia[F, G, C](
      nodeId,
      alpha,
      pingExpiresIn,
      _ ⇒ true.pure[IO],
      IO.pure(ownContactValue),
      contact ⇒
        new KademliaRpc[C] {
          val kad = getKademlia(contact)

          /**
           * Ping the contact, get its actual Node status, or fail
           */
          override def ping() =
            toIO(kad.update(ownContactValue).flatMap(_ ⇒ kad.handleRPC.ping().to[F]))

          /**
           * Perform a local lookup for a key, return K closest known nodes
           *
           * @param key Key to lookup
           */
          override def lookup(key: Key, numberOfNodes: Int) =
            toIO(kad.update(ownContactValue).flatMap(_ ⇒ kad.handleRPC.lookup(key, numberOfNodes).to[F]))

          /**
           * Perform a local lookup for a key, return K closest known nodes, going away from the second key
           *
           * @param key Key to lookup
           */
          override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int) =
            toIO(
              kad.update(ownContactValue).flatMap(_ ⇒ kad.handleRPC.lookupAway(key, moveAwayFrom, numberOfNodes).to[F])
            )

      }
    )
  }

  def coeval[C](
    nodeId: Key,
    alpha: Int,
    k: Int,
    getKademlia: C ⇒ Kademlia[Coeval, C],
    toContact: Key ⇒ C,
    pingExpiresIn: FiniteDuration = 1.second
  ): Kademlia[Coeval, C] = {
    implicit val bucketOps: Bucket.WriteOps[Coeval, C] = new TestBucketOps[C](k)
    implicit val siblingsOps: Siblings.WriteOps[Coeval, C] = new TestSiblingOps[C](nodeId, k)

    TestKademlia[Coeval, Coeval, C](nodeId, alpha, k, getKademlia, toContact, new (Coeval ~> IO) {
      override def apply[A](fa: Coeval[A]): IO[A] = fa.toIO
    }, pingExpiresIn)
  }

  def coevalSimulation[C](
    k: Int,
    n: Int,
    toContact: Key ⇒ C,
    nextRandomKey: ⇒ Key,
    joinPeers: Int = 0,
    alpha: Int = 3,
    pingExpiresIn: FiniteDuration = 1.second
  ): Map[C, Kademlia[Coeval, C]] = {
    lazy val kads: Map[C, Kademlia[Coeval, C]] =
      Stream
        .fill(n)(nextRandomKey)
        .foldLeft(Map.empty[C, Kademlia[Coeval, C]]) {
          case (acc, key) ⇒
            acc + (toContact(key) -> TestKademlia.coeval(key, alpha, k, kads(_), toContact, pingExpiresIn))
        }

    val peers = kads.keys.take(joinPeers).toSeq

    if (peers.nonEmpty)
      kads.values.foreach(_.join(peers, k).run.value)

    kads
  }

  def coevalSimulationKP[C](
    k: Int,
    n: Int,
    toContact: Key ⇒ C,
    nextRandomKeyPair: ⇒ KeyPair,
    joinPeers: Int = 0,
    alpha: Int = 3,
    pingExpiresIn: FiniteDuration = 1.second
  ): Map[C, (Signer, Kademlia[Coeval, C])] = {
    lazy val kads: Map[C, (Signer, Kademlia[Coeval, C])] =
      Stream
        .fill(n)(nextRandomKeyPair)
        .foldLeft(Map.empty[C, (Signer, Kademlia[Coeval, C])]) {
          case (acc, keyPair) ⇒
            val algo = SignAlgo.dumb
            val signer = algo.signer(keyPair)
            val key = Key.fromPublicKey[Coeval](keyPair.publicKey).value
            acc + (toContact(key) -> (signer, TestKademlia.coeval(key, alpha, k, kads(_)._2, toContact, pingExpiresIn)))
        }

    val peers = kads.keys.take(joinPeers).toSeq

    if (peers.nonEmpty)
      kads.values.foreach(_._2.join(peers, k).run.value)

    kads
  }
}
