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

import java.nio.file.Path

import cats.data.EitherT
import cats.syntax.functor._
import cats.syntax.compose._
import cats.syntax.flatMap._
import cats.syntax.applicative._
import cats.instances.list._
import cats.instances.option._
import cats.{Defer, Monad, MonadError, Parallel, Traverse}
import cats.effect.{Clock, Concurrent, ConcurrentEffect, ContextShift, LiftIO, Resource, Timer}
import com.softwaremill.sttp.SttpBackend
import fluence.codec.{CodecError, PureCodec}
import fluence.crypto.{Crypto, KeyPair}
import fluence.crypto.signature.SignAlgo
import fluence.effects.kvstore.{KVStore, RocksDBStore}
import fluence.kad.{Kademlia, RoutingConf}
import fluence.kad.http.{KademliaHttp, KademliaHttpClient, UriContact}
import fluence.kad.protocol.{ContactAccess, Key, Node}
import fluence.kad.routing.RoutingTable
import fluence.log.Log
import fluence.node.config.KademliaConfig

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

case class KademliaNode[F[_], C](
  kademlia: Kademlia[F, C],
  http: KademliaHttp[F, C]
)

object KademliaNode {

  def make[F[_]: ConcurrentEffect: Timer: Log: ContextShift, P[_]](
    conf: KademliaConfig,
    signAlgo: SignAlgo,
    keyPair: KeyPair,
    rootPath: Path
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

      cacheExt ← cacheExtResource[F, UriContact](conf.routing, rootPath)
      refreshingExt ← refreshingExtResource[F, UriContact](conf.routing)

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
      _ ← joinConcurrently(kad, conf.join, UriContact.readAndCheckContact(signAlgo.checker))

      http = new KademliaHttp[F, UriContact](kad, readNode, writeNode)
    } yield KademliaNode(kad, http)
  }

  private def joinConcurrently[F[_]: Concurrent: Log, C](kad: Kademlia[F, C],
                                                         conf: KademliaConfig.Join,
                                                         readContact: Crypto.Func[String, C]): Resource[F, Unit] =
    Resource
      .make(
        Concurrent[F].start(
          Traverse[List]
            .traverse(
              conf.seeds.toList
            )(
              s ⇒
                readContact
                  .runEither[F](s)
                  .flatTap(
                    r =>
                      Traverse[Option].traverse(r.left.toOption)(e => Log[F].info(s"Filtered out kademlia seed $s: $e"))
                )
            )
            .map(_.collect {
              case Right(c) ⇒ c
            })
            .flatMap(
              seeds ⇒
                Log[F].scope("kad" -> "join")(
                  log ⇒
                    kad.join(seeds, conf.numOfNodes)(log).flatMap {
                      case true ⇒
                        log.info("Joined")
                      case false ⇒
                        log.warn("Unable to join any Kademlia seed")
                  }
              )
            )
        )
      )(_.cancel)
      .void

  private def refreshingExtResource[F[_]: Concurrent: Timer: Log, C](
    conf: RoutingConf
  ): Resource[F, Option[RoutingTable.Extension[F, C]]] =
    Resource.pure(
      conf.refreshing
        .filter(_.period.isFinite())
        .fold[Option[RoutingTable.Extension[F, C]]](None)(
          r ⇒
            Some(
              RoutingTable.refreshing[F, C](
                r.period.asInstanceOf[FiniteDuration], // Checked with .filter
                r.neighbors,
                conf.parallelism
              )
          )
        )
    )

  private def cacheExtResource[F[_]: LiftIO: ContextShift: Log: Concurrent: Clock, C](
    conf: RoutingConf,
    rootPath: Path
  )(implicit
    ca: ContactAccess[F, C],
    writeNode: PureCodec.Func[Node[C], String],
    readNode: Crypto.Func[String, Node[C]]): Resource[F, Option[RoutingTable.Extension[F, C]]] =
    conf.store
      .fold(Resource.pure[F, Option[RoutingTable.Extension[F, C]]](None)) { cachePath ⇒
        val nodeCodec: PureCodec[String, Node[C]] =
          PureCodec.build(
            PureCodec.fromOtherFunc(readNode)(ee ⇒ CodecError("Cannot decode Node due to Crypto error", Some(ee))),
            writeNode
          )

        implicit val nodeBytesCodec: PureCodec[Array[Byte], Node[C]] =
          nodeCodec compose PureCodec
            .liftB[Array[Byte], String](bs ⇒ new String(bs), _.getBytes())

        makeCache[F, C](rootPath, cachePath)
          .map(RoutingTable.bootstrapWithStore[F, C](_))
          .map(Some(_))
      }

  private def makeCache[F[_]: Monad: Defer: LiftIO: ContextShift: Log, C](
    rootPath: Path,
    cachePath: String
  )(implicit nodeCodec: PureCodec[Array[Byte], Node[C]]): Resource[F, KVStore[F, Key, Node[C]]] =
    RocksDBStore.make[F, Key, Node[C]](rootPath.resolve(cachePath).toAbsolutePath.toString)

}
