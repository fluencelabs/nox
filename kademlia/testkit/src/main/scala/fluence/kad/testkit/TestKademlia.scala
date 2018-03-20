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

class TestKademlia[F[_], C](
  nodeId: Key,
  alpha: Int,
  k: Int,
  getKademlia: C ⇒ Kademlia[F, C],
  toContact: Key ⇒ C,
  pingExpiresIn: FiniteDuration = 1.second)(
  implicit
  BW: Bucket.WriteOps[F, C],
  SW: Siblings.WriteOps[F, C],
  ME: MonadError[F, Throwable],
  P: Parallel[F, F])
    extends Kademlia[F, C](nodeId, alpha, pingExpiresIn, _ ⇒ true.pure[F]) {

  def ownContactValue = Node[C](nodeId, Instant.now(), toContact(nodeId))

  override def ownContact: F[Node[C]] = ME.pure(ownContactValue)

  override def rpc(contact: C): KademliaRpc[F, C] = new KademliaRpc[F, C] {
    val kad = getKademlia(contact)

    /**
     * Ping the contact, get its actual Node status, or fail
     */
    override def ping() =
      kad.update(ownContactValue).flatMap(_ ⇒ kad.handleRPC.ping())

    /**
     * Perform a local lookup for a key, return K closest known nodes
     *
     * @param key Key to lookup
     */
    override def lookup(key: Key, numberOfNodes: Int) =
      kad.update(ownContactValue).flatMap(_ ⇒ kad.handleRPC.lookup(key, numberOfNodes))

    /**
     * Perform a local lookup for a key, return K closest known nodes, going away from the second key
     *
     * @param key Key to lookup
     */
    override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int) =
      kad.update(ownContactValue).flatMap(_ ⇒ kad.handleRPC.lookupAway(key, moveAwayFrom, numberOfNodes))

  }

}

object TestKademlia {
  implicit val CoevalParallel: Parallel[Coeval, Coeval] = new Parallel[Coeval, Coeval] {
    override def applicative = Applicative[Coeval]

    override def monad = Monad[Coeval]

    override def sequential: Coeval ~> Coeval = new (Coeval ~> Coeval) {
      override def apply[A](fa: Coeval[A]) = fa
    }

    override def parallel = sequential
  }

  def coeval[C](
    nodeId: Key,
    alpha: Int,
    k: Int,
    getKademlia: C ⇒ Kademlia[Coeval, C],
    toContact: Key ⇒ C,
    pingExpiresIn: FiniteDuration = 1.second): Kademlia[Coeval, C] =
    new TestKademlia[Coeval, C](nodeId, alpha, k, getKademlia, toContact, pingExpiresIn)(
      ME = implicitly[MonadError[Coeval, Throwable]],
      BW = new TestBucketOps[C](k),
      SW = new TestSiblingOps[C](nodeId, k),
      P = CoevalParallel)

  def coevalSimulation[C](
    k: Int,
    n: Int,
    toContact: Key ⇒ C,
    nextRandomKey: ⇒ Key,
    joinPeers: Int = 0,
    alpha: Int = 3,
    pingExpiresIn: FiniteDuration = 1.second): Map[C, Kademlia[Coeval, C]] = {
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
    pingExpiresIn: FiniteDuration = 1.second): Map[C, (Signer, Kademlia[Coeval, C])] = {
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
