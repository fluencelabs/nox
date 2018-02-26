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

package fluence.client

import cats.effect.{ Effect, IO }
import cats.kernel.Monoid
import com.typesafe.config.Config
import fluence.client.config.{ KademliaConfigParser, KeyPairConfig, SeedsConfig }
import fluence.crypto.hash.CryptoHasher
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.SignatureChecker
import fluence.crypto.{ FileKeyStorage, SignAlgo }
import fluence.dataset.BasicContract
import fluence.dataset.client.Contracts
import fluence.dataset.grpc.DatasetStorageClient
import fluence.dataset.grpc.client.{ ContractAllocatorClient, ContractsCacheClient }
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsCacheRpc }
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.protocol.{ Contact, KademliaRpc, Key }
import fluence.kad.{ Kademlia, KademliaConf, KademliaMVar }
import fluence.transport.TransportSecurity
import fluence.transport.grpc.client.GrpcClient
import monix.eval.Task
import monix.execution.Scheduler
import shapeless.HNil

import scala.language.higherKinds

object ClientComposer extends slogging.LazyLogging {

  /**
   * Register all Rpc's into [[fluence.transport.TransportClient]] and returns it.
   */
  def grpc[F[_] : Effect](
    builder: GrpcClient.Builder[HNil]
  )(implicit checker: SignatureChecker, scheduler: Scheduler = Scheduler.global) = {

    import fluence.dataset.grpc.BasicContractCodec.{ codec ⇒ contractCodec }
    import fluence.kad.grpc.KademliaNodeCodec.{ codec ⇒ nodeCodec }

    builder
      .add(KademliaClient.register[F]())
      .add(ContractsCacheClient.register[F, BasicContract]())
      .add(ContractAllocatorClient.register[F, BasicContract]())
      .add(DatasetStorageClient.register[F]())
      .build
  }

  def buildClient(config: Config, signAlgo: SignAlgo, cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]])(implicit scheduler: Scheduler = Scheduler.global): IO[FluenceClient] =
    for {
      seedsConf ← SeedsConfig.read(config)
      contacts ← seedsConf.contacts(signAlgo.checker)
      client ← fluenceClient(contacts, signAlgo, cryptoHasher, config)
    } yield client

  def getKeyPair(config: Config, algo: SignAlgo): IO[KeyPair] =
    for {
      kpConf ← KeyPairConfig.read(config)
      kp ← FileKeyStorage.getKeyPair[IO](kpConf.keyPath, algo)
    } yield kp

  private def fluenceClient(
    seeds: Seq[Contact],
    signAlgo: SignAlgo,
    storageHasher: CryptoHasher[Array[Byte], Array[Byte]],
    config: Config
  )(implicit scheduler: Scheduler = Scheduler.global): IO[FluenceClient] = {

    import signAlgo.checker
    val client = ClientComposer.grpc[Task](GrpcClient.builder)
    val kademliaRpc = client.service[KademliaRpc[Task, Contact]] _
    for {
      conf ← KademliaConfigParser.readKademliaConfig[IO](config)
      _ = logger.info("Create kademlia client.")
      kademliaClient = createKademliaClient(conf, kademliaRpc)
      _ = logger.info("Connecting to seed node.")
      _ ← kademliaClient.join(seeds, 2).toIO
      _ = logger.info("Create contracts api.")
      contracts = new Contracts[Task, BasicContract, Contact](
        maxFindRequests = 10,
        maxAllocateRequests = _ ⇒ 20,
        kademlia = kademliaClient,
        cacheRpc = contact ⇒ client.service[ContractsCacheRpc[Task, BasicContract]](contact),
        allocatorRpc = contact ⇒ client.service[ContractAllocatorRpc[Task, BasicContract]](contact)
      )
    } yield FluenceClient(kademliaClient, contracts, client.service[DatasetStorageRpc[Task]], signAlgo, storageHasher, config)
  }

  private def createKademliaClient(conf: KademliaConf, kademliaRpc: Contact ⇒ KademliaRpc[Task, Contact]): Kademlia[Task, Contact] = {
    val check = TransportSecurity.canBeSaved[Task](Monoid.empty[Key], acceptLocal = true)
    KademliaMVar.client(kademliaRpc, conf, check)
  }

}
