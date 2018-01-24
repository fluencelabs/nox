package fluence.node

import cats.~>
import cats.instances.try_._
import fluence.client.ClientComposer
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{ SignatureChecker, Signer }
import fluence.dataset.BasicContract
import fluence.dataset.grpc.{ ContractAllocatorGrpc, ContractsCacheGrpc, DatasetContractsApiGrpc }
import fluence.dataset.grpc.server.{ ContractAllocatorServer, ContractsApiServer, ContractsCacheServer }
import fluence.dataset.node.Contracts
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsCacheRpc }
import fluence.kad.grpc.KademliaGrpc
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.protocol.{ Contact, Key }
import fluence.storage.TrieMapKVStore
import fluence.transport.TransportSecurity
import fluence.transport.grpc.client.GrpcClient
import fluence.transport.grpc.server.{ GrpcServer, GrpcServerConf }
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global

import scala.concurrent.Future
import scala.concurrent.duration._
import scala.util.Try

class NodeComposer(keyPair: KeyPair, conf: GrpcServerConf = GrpcServerConf.read()) {

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

  private val serverBuilder = GrpcServer.builder

  val key = Key.fromKeyPair[Try](keyPair).get

  // We have only Kademlia service client
  val client = ClientComposer.grpc[Task](GrpcClient.builder(key, serverBuilder.contact))

  // This is a server service, not a client
  val kad = new KademliaService(
    key,
    serverBuilder.contact,
    client.service[KademliaClient[Task]],
    KademliaConf.read(),
    TransportSecurity.canBeSaved[Task](key, acceptLocal = GrpcServerConf.read().acceptLocal)
  )

  val contractsApi = new Contracts[Task, BasicContract, Contact](
    nodeId = key,
    storage = TrieMapKVStore(), // TODO: cache persistence
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

  // Add server (with kademlia inside), build
  val server = serverBuilder
    .add(KademliaGrpc.bindService(new KademliaServer[Task](kad.handleRPC), global))
    .add(ContractsCacheGrpc.bindService(new ContractsCacheServer[Task, BasicContract](cache = contractsApi.cache), global))
    .add(ContractAllocatorGrpc.bindService(new ContractAllocatorServer[Task, BasicContract](contractAllocator = contractsApi.allocator), global))
    .add(DatasetContractsApiGrpc.bindService(new ContractsApiServer[Task, BasicContract](api = contractsApi), global))
    .onNodeActivity(kad.update)
    .build

}
