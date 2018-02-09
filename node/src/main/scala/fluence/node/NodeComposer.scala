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

package fluence.node

import java.util.concurrent.Executors

import cats.instances.try_._
import cats.~>
import com.typesafe.config.Config
import fluence.client.ClientComposer
import fluence.crypto.SignAlgo
import fluence.crypto.hash.JdkCryptoHasher
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signer
import fluence.dataset.BasicContract
import fluence.dataset.grpc.server.{ ContractAllocatorServer, ContractsApiServer, ContractsCacheServer }
import fluence.dataset.grpc.storage.DatasetStorageRpcGrpc
import fluence.dataset.grpc.{ ContractAllocatorGrpc, ContractsCacheGrpc, DatasetContractsApiGrpc, DatasetStorageServer }
import fluence.dataset.node.Contracts
import fluence.dataset.node.storage.Datasets
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsApi, ContractsCacheRpc }
import fluence.info.grpc.{ NodeInfoRpcGrpc, NodeInfoServer }
import fluence.info.{ NodeInfo, NodeInfoRpc, NodeInfoService }
import fluence.kad.Kademlia
import fluence.kad.grpc.KademliaGrpc
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.protocol.{ Contact, Key }
import fluence.transport.TransportSecurity
import fluence.transport.grpc.client.{ GrpcClient, GrpcClientConf }
import fluence.transport.grpc.server.{ GrpcServer, GrpcServerConf }
import monix.eval.Task
import monix.execution.Scheduler

import scala.concurrent.{ ExecutionContext, Future }
import scala.concurrent.duration._

class NodeComposer(
    keyPair: KeyPair,
    algo: SignAlgo,
    config: Config,
    getInfo: () ⇒ Task[NodeInfo],
    contractsCacheStoreName: String = "fluence_contractsCache") {

  private val ec = ExecutionContext.fromExecutorService(Executors.newCachedThreadPool())

  private implicit val runFuture = new (Future ~> Task) {
    override def apply[A](fa: Future[A]): Task[A] = Task.deferFuture(fa)
  }

  private implicit val runTask = new (Task ~> Future) {
    // TODO: add logging
    override def apply[A](fa: Task[A]): Future[A] = fa.runAsync(Scheduler.global)
  }

  private implicit def runId[F[_]] = new (F ~> F) {
    override def apply[A](fa: F[A]): F[A] = fa
  }

  private implicit val kadCodec = fluence.kad.grpc.KademliaNodeCodec[Task]
  private implicit val contractCodec = fluence.dataset.grpc.BasicContractCodec.codec[Task]
  private val keyC = Key.bytesCodec[Task]

  import keyC.inverse

  private lazy val serverBuilder =
    grpcConf.map(GrpcServer.builder(_)).memoizeOnSuccess

  private lazy val grpcConf =
    GrpcServerConf.read[Task](config).memoizeOnSuccess

  private lazy val grpcClientConf =
    GrpcClientConf.read[Task](config).memoizeOnSuccess

  lazy val services: Task[NodeServices[Task, BasicContract, Contact]] =
    (for {
      k ← Key.fromKeyPair[Task](keyPair)
      conf ← grpcConf

      serverBuilder ← serverBuilder

      contractsCacheStore ← ContractsCacheStore[Task](contractsCacheStoreName, config)

      kadConf ← KademliaConf.read[Task](config)

      clientConf ← grpcClientConf

      // TODO: externalize creation of signer/checker somehow

    } yield new NodeServices[Task, BasicContract, Contact] {

      private val client = ClientComposer.grpc[Task](GrpcClient.builder(k, serverBuilder.contact, clientConf))

      override val key: Key = k

      override val signer: Signer[Task] = algo.signer(keyPair)

      override lazy val kademlia: Kademlia[Task, Contact] = new KademliaService(
        k,
        serverBuilder.contact,
        client.service[KademliaClient[Task]],
        kadConf,
        TransportSecurity.canBeSaved[Task](k, acceptLocal = conf.acceptLocal)
      )

      private lazy val contractsApi = new Contracts[Task, BasicContract, Contact](
        nodeId = k,
        storage = contractsCacheStore,
        createDataset = _ ⇒ Task.unit, // TODO: dataset creation
        checkAllocationPossible = _ ⇒ Task.unit, // TODO: check allocation possible
        maxFindRequests = 100,
        maxAllocateRequests = n ⇒ 30 * n,
        checker = algo.checker[Task],
        signer = signer,
        cacheTtl = 1.day,
        kademlia = kademlia
      ) {
        override def cacheRpc(contact: Contact): ContractsCacheRpc[Task, BasicContract] =
          client.service[ContractsCacheRpc[Task, BasicContract]](contact)

        override def allocatorRpc(contact: Contact): ContractAllocatorRpc[Task, BasicContract] =
          client.service[ContractAllocatorRpc[Task, BasicContract]](contact)
      }

      override lazy val contracts: ContractsApi[Task, BasicContract] = contractsApi

      override lazy val contractsCache: ContractsCacheRpc[Task, BasicContract] = contractsApi.cache

      override lazy val contractAllocator: ContractAllocatorRpc[Task, BasicContract] = contractsApi.allocator

      override lazy val info: NodeInfoRpc[Task] = new NodeInfoService[Task](getInfo)

      override lazy val datasets: DatasetStorageRpc[Task] =
        new Datasets(
          config,
          JdkCryptoHasher.Sha256, // TODO: externalize hasher
          contractsCacheStore.get(_).map(_.contract.participants.contains(k))
        )
    }).memoizeOnSuccess

  // Add server (with kademlia inside), build
  lazy val server: Task[GrpcServer] =
    (
      for {
        serverBuilder ← serverBuilder
        ns ← services
        clientConf ← grpcClientConf
      } yield {
        import ns._
        serverBuilder
          .add(KademliaGrpc.bindService(new KademliaServer[Task](kademlia.handleRPC), ec))
          .add(ContractsCacheGrpc.bindService(new ContractsCacheServer[Task, BasicContract](contractsCache), ec))
          .add(ContractAllocatorGrpc.bindService(new ContractAllocatorServer[Task, BasicContract](contractAllocator), ec))
          .add(DatasetContractsApiGrpc.bindService(new ContractsApiServer[Task, BasicContract](contracts), ec))
          .add(NodeInfoRpcGrpc.bindService(new NodeInfoServer[Task](info), ec))
          .add(DatasetStorageRpcGrpc.bindService(new DatasetStorageServer[Task](datasets), ec))
          .onNodeActivity(kademlia.update _, clientConf)
          .build
      }
    ).memoizeOnSuccess

}
