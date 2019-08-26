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

import java.nio.file.Path

import cats.data.EitherT
import cats.effect._
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Monad, Parallel}
import com.softwaremill.sttp.SttpBackend
import fluence.crypto.signature.SignAlgo
import fluence.crypto.{Crypto, KeyPair}
import fluence.kad.Kademlia
import fluence.kad.conf.KademliaConfig
import fluence.kad.contact.{ContactAccess, UriContact}
import fluence.kad.protocol.Node
import fluence.kad.routing.RoutingTable
import fluence.log.Log

import scala.language.higherKinds

/**
 * Kademlia HTTP node.
 * Run it on your own node to get access to the Kademlia network using HTTP transport.
 *
 * @param kademlia Node's Kademlia instance
 * @param http Node's HTTP routes, you are responsible for running a server with them
 * @param joinFiber Background fiber for Kademlia JOIN; use it to wait for join to complete
 * @tparam F Effect
 * @tparam C Contact
 */
case class KademliaHttpNode[F[_], C] private (
  kademlia: Kademlia[F, C],
  http: KademliaHttp[F, C],
  joinFiber: Fiber[F, Unit]
)

object KademliaHttpNode {

  /**
   * Make Kademlia instance with associated [[KademliaHttp]] routes, using [[UriContact]] for contacts.
   *
   * @param conf Kademlia config
   * @param signAlgo Signing and signature checking algorithm for contact serialization
   * @param keyPair This node's keypair, to be used to sign contacts with signAlgo
   * @param rootPath RocksDB storage root path
   * @param nodeCodec Mean to encode, decode, sign and check Node[UriContact]
   */
  def make[F[_]: ConcurrentEffect: Timer: Log: ContextShift, P[_]](
    conf: KademliaConfig,
    signAlgo: SignAlgo,
    keyPair: KeyPair,
    rootPath: Path,
    nodeCodec: UriContact.NodeCodec
  )(
    implicit
    P: Parallel[F, P],
    sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing]
  ): Resource[F, KademliaHttpNode[F, UriContact]] = {

    implicit val readNode: Crypto.Func[String, Node[UriContact]] =
      nodeCodec.readNode(signAlgo.checker)

    import Crypto.liftCodecErrorToCrypto
    import nodeCodec.writeNode

    val nodeAuth = for {
      selfNode ← nodeCodec.buildNode(conf.advertize, signAlgo.signer(keyPair))
      selfNodeAuth ← Crypto.fromOtherFunc(writeNode).pointAt(selfNode)
    } yield (selfNode, selfNodeAuth)

    for {
      // Prepare self-referencing Node[C] and contact string
      (selfNode, contactStr) ← Resource.liftF(nodeAuth.runF[F](()))

      _ ← Log.resource[F].info(s"Kademlia Seed: $contactStr")

      // Express the way we're going to access other nodes
      implicit0(ca: ContactAccess[F, UriContact]) ← new ContactAccess[F, UriContact](
        pingExpiresIn = conf.routing.pingExpiresIn,
        _ ⇒ Monad[F].pure(true), // TODO implement formal check function
        (contact: UriContact) ⇒ new KademliaHttpClient(contact.host, contact.port, contactStr)
      ).pure[Resource[F, ?]]

      cacheExt ← RoutingTable.rocksdbStoreExtResource[F, UriContact](conf.routing, rootPath)
      refreshingExt = RoutingTable.refreshingOpt[F, UriContact](conf.routing)

      // Initiate an empty RoutingTable, bootstrap it from store if extension is enabled
      rt ← Resource.liftF(
        RoutingTable[F, P, UriContact](
          selfNode.key,
          siblingsSize = conf.routing.maxSiblingsSize,
          maxBucketSize = conf.routing.maxBucketSize,
          extensions = cacheExt.toList ::: refreshingExt.toList
        )
      )

      // Kademlia instance just wires everything together
      kad = Kademlia[F, P, UriContact](rt, selfNode.pure[F], conf.routing)

      // Join Kademlia network in a separate fiber
      joinFiber ← Kademlia
        .joinConcurrently(kad, conf.join, UriContact.readAndCheckContact(signAlgo.checker))

      http = new KademliaHttp[F, UriContact](kad, readNode, writeNode)
    } yield KademliaHttpNode(kad, http, joinFiber)
  }

}
