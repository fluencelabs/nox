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
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.applicative._
import cats.{Monad, MonadError, Parallel}
import cats.effect.{Clock, Effect, Resource}
import com.softwaremill.sttp.SttpBackend
import fluence.crypto.{Crypto, KeyPair}
import fluence.crypto.signature.SignAlgo
import fluence.kad.{Kademlia, KademliaConf}
import fluence.kad.http.{KademliaHttp, KademliaHttpClient, UriContact}
import fluence.kad.protocol.{ContactAccess, Node}
import fluence.kad.routing.RoutingTable
import fluence.log.Log

import scala.language.higherKinds

case class KademliaNode[F[_], C](
  kademlia: Kademlia[F, C],
  http: KademliaHttp[F, C]
)

object KademliaNode {

  def make[F[_]: Effect: Clock: Log, P[_]](
    host: String,
    port: Short,
    conf: KademliaConf,
    signAlgo: SignAlgo,
    keyPair: KeyPair
  )(
    implicit
    P: Parallel[F, P],
    sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing],
    ME: MonadError[F, Throwable] // That's to fail fast if we're not able to create a contact
  ): Resource[F, KademliaNode[F, UriContact]] = {

    implicit val readNode: Crypto.Func[String, Node[UriContact]] =
      UriContact.readNode(signAlgo.checker)

    import UriContact.writeNode
    import Crypto.liftCodecErrorToCrypto

    val nodeAuth = for {
      selfNode ← UriContact.buildNode(host, port, signAlgo.signer(keyPair))
      selfNodeAuth ← Crypto.fromOtherFunc(writeNode).pointAt(selfNode)
    } yield (selfNode, selfNodeAuth)

    val makeNodeF = for {
      // Prepare self-referencing Node[C] and contact string
      (selfNode, contactStr) ← nodeAuth.runF[F](())

      _ ← Log[F].info(s"Kademlia Seed: $contactStr")

      // Express the way we're going to access other nodes
      implicit0(ca: ContactAccess[F, UriContact]) ← new ContactAccess[F, UriContact](
        pingExpiresIn = conf.pingExpiresIn,
        _ ⇒ Monad[F].pure(true), // TODO implement formal check function
        (contact: UriContact) ⇒ new KademliaHttpClient(contact.host, contact.port, contactStr)
      ).pure[F]

      // Initiate an empty RoutingTable
      rt ← RoutingTable[F, P, UriContact](
        selfNode.key,
        siblingsSize = conf.maxSiblingsSize,
        maxBucketSize = conf.maxBucketSize
      )

      // Kademlia instance just wires everything together
      kad = Kademlia[F, P, UriContact](rt, selfNode.pure[F], conf)
      http = new KademliaHttp[F, UriContact](kad, readNode, writeNode)
    } yield KademliaNode(kad, http)

    // Joining should be performed later with some seeds
    Resource.liftF[F, KademliaNode[F, UriContact]](makeNodeF)
  }

}
