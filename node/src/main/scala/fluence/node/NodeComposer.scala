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

import cats.instances.try_._
import cats.~>
import fluence.client.ClientComposer
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{ SignatureChecker, Signer }
import fluence.dataset.BasicContract
import fluence.dataset.grpc.server.{ ContractAllocatorServer, ContractsApiServer, ContractsCacheServer }
import fluence.dataset.grpc.{ ContractAllocatorGrpc, ContractsCacheGrpc, DatasetContractsApiGrpc }
import fluence.dataset.node.Contracts
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsCacheRpc }
import fluence.info.{ NodeInfo, NodeInfoService }
import fluence.info.grpc.{ NodeInfoRpcGrpc, NodeInfoServer }
import fluence.kad.grpc.KademliaGrpc
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.protocol.{ Contact, Key }
import fluence.transport.TransportSecurity
import fluence.transport.grpc.client.GrpcClient
import fluence.transport.grpc.server.{ GrpcServer, GrpcServerConf }
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global

import scala.concurrent.Future
import scala.concurrent.duration._
import scala.util.Try

class NodeComposer(
    keyPair: KeyPair,
    getInfo: () ⇒ Task[NodeInfo],
    conf: GrpcServerConf = GrpcServerConf.read(),
    contractsCacheStoreName: String = "fluence_contractsCache") {

  private implicit val runFuture = new (Future ~> Task) {
    override def apply[A](fa: Future[A]): Task[A] = Task.deferFuture(fa)
  }

  private implicit val runTask = new (Task ~> Future) {
    // TODO: add logging
    override def apply[A](fa: Task[A]): Future[A] = fa.runAsync
  }

  private implicit val kadCodec = fluence.kad.grpc.KademliaNodeCodec[Task]
  private implicit val contractCodec = fluence.dataset.grpc.BasicContractCodec.codec[Task]
  private val keyC = Key.bytesCodec[Task]

  import keyC.inverse

  private val serverBuilder = GrpcServer.builder(conf)

  val key = Key.fromKeyPair[Try](keyPair).get

  // We have only Kademlia service client
  val client = ClientComposer.grpc[Task](GrpcClient.builder(key, serverBuilder.contact))

  // This is a server service, not a client
  val kad = new KademliaService(
    key,
    serverBuilder.contact,
    client.service[KademliaClient[Task]],
    KademliaConf.read(),
    TransportSecurity.canBeSaved[Task](key, acceptLocal = conf.acceptLocal)
  )

  val contractsApi = new Contracts[Task, BasicContract, Contact](
    nodeId = key,
    storage = ContractsCacheStore(contractsCacheStoreName),
    createDataset = _ ⇒ Task.unit, // TODO: dataset creation
    checkAllocationPossible = _ ⇒ Task.unit, // TODO: check allocation possible
    maxFindRequests = 100,
    maxAllocateRequests = n ⇒ 30 * n,
    checker = SignatureChecker.DumbChecker,
    signer = new Signer.DumbSigner(keyPair),
    cacheTtl = 1.day,
    kademlia = kad
  ) {
    override def cacheRpc(contact: Contact): ContractsCacheRpc[Task, BasicContract] =
      client.service[ContractsCacheRpc[Task, BasicContract]](contact)

    override def allocatorRpc(contact: Contact): ContractAllocatorRpc[Task, BasicContract] =
      client.service[ContractAllocatorRpc[Task, BasicContract]](contact)
  }

  val info = new NodeInfoService[Task](getInfo)

  // Add server (with kademlia inside), build
  val server = serverBuilder
    .add(KademliaGrpc.bindService(new KademliaServer[Task](kad.handleRPC), global))
    .add(ContractsCacheGrpc.bindService(new ContractsCacheServer[Task, BasicContract](contractsApi.cache), global))
    .add(ContractAllocatorGrpc.bindService(new ContractAllocatorServer[Task, BasicContract](contractsApi.allocator), global))
    .add(DatasetContractsApiGrpc.bindService(new ContractsApiServer[Task, BasicContract](contractsApi), global))
    .add(NodeInfoRpcGrpc.bindService(new NodeInfoServer[Task](info), global))
    .onNodeActivity(kad.update)
    .build

}
