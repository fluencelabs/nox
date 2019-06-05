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
import cats.instances.list._
import cats.{Monad, MonadError, Parallel, Traverse}
import cats.effect.{Clock, Concurrent, ConcurrentEffect, Resource}
import com.softwaremill.sttp.SttpBackend
import fluence.crypto.{Crypto, KeyPair}
import fluence.crypto.signature.SignAlgo
import fluence.kad.Kademlia
import fluence.kad.http.{KademliaHttp, KademliaHttpClient, UriContact}
import fluence.kad.protocol.{ContactAccess, Node}
import fluence.kad.routing.RoutingTable
import fluence.log.Log
import fluence.node.config.KademliaConfig

import scala.language.higherKinds

case class KademliaNode[F[_], C](
  kademlia: Kademlia[F, C],
  http: KademliaHttp[F, C]
)

object KademliaNode {

  def make[F[_]: ConcurrentEffect: Clock: Log, P[_]](
    conf: KademliaConfig,
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
      selfNode ← UriContact.buildNode(conf.advertize.host, conf.advertize.port, signAlgo.signer(keyPair))
      selfNodeAuth ← Crypto.fromOtherFunc(writeNode).pointAt(selfNode)
    } yield (selfNode, selfNodeAuth)

    val makeNodeF = for {
      // Prepare self-referencing Node[C] and contact string
      (selfNode, contactStr) ← nodeAuth.runF[F](())

      _ ← Log[F].info(s"Kademlia Seed: $contactStr")

      // Express the way we're going to access other nodes
      implicit0(ca: ContactAccess[F, UriContact]) ← new ContactAccess[F, UriContact](
        pingExpiresIn = conf.routing.pingExpiresIn,
        _ ⇒ Monad[F].pure(true), // TODO implement formal check function
        (contact: UriContact) ⇒ new KademliaHttpClient(contact.host, contact.port, contactStr)
      ).pure[F]

      // Initiate an empty RoutingTable
      rt ← RoutingTable[F, P, UriContact](
        selfNode.key,
        siblingsSize = conf.routing.maxSiblingsSize,
        maxBucketSize = conf.routing.maxBucketSize
      )

      // Kademlia instance just wires everything together
      kad = Kademlia[F, P, UriContact](rt, selfNode.pure[F], conf.routing)

      // Join asynchronously
      fiber ← Concurrent[F].start(
        Traverse[List]
          .traverse(
            conf.join.seeds.toList
          )(s ⇒ UriContact.readAndCheckContact(signAlgo.checker).runEither[F](s))
          .map(_.collect {
            case Right(c) ⇒ c
          })
          .flatMap(
            seeds ⇒
              Log[F].scope("kad" -> "join")(
                log ⇒
                  kad.join(seeds, conf.join.numOfNodes)(log).flatMap {
                    case true ⇒
                      log.info("Joined")
                    case false ⇒
                      log.warn("Unable to join any Kademlia seed")
                }
            )
          )
      )

      http = new KademliaHttp[F, UriContact](kad, readNode, writeNode)
    } yield KademliaNode(kad, http) -> fiber

    Resource.make(makeNodeF)(_._2.cancel).map(_._1)
  }

}
