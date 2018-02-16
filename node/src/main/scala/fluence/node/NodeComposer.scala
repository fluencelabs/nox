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

import java.time.Instant
import java.util.concurrent.Executors

import cats.effect.IO
import cats.{ ApplicativeError, MonadError, ~> }
import com.typesafe.config.Config
import fluence.client.ClientComposer
import fluence.crypto.SignAlgo
import fluence.crypto.hash.{ CryptoHasher, JdkCryptoHasher }
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{ SignatureChecker, Signer }
import fluence.dataset.BasicContract
import fluence.dataset.grpc.server.{ ContractAllocatorServer, ContractsCacheServer }
import fluence.dataset.grpc.storage.DatasetStorageRpcGrpc
import fluence.dataset.grpc.{ ContractAllocatorGrpc, ContractsCacheGrpc, DatasetStorageServer }
import fluence.dataset.node.contract.ContractRecord
import fluence.dataset.node.storage.Datasets
import fluence.dataset.node.{ ContractAllocator, ContractsCache }
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsCacheRpc }
import fluence.kad.grpc.KademliaGrpc
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.protocol.{ Contact, Key }
import fluence.kad.{ Kademlia, KademliaConf, KademliaMVar }
import fluence.storage.KVStore
import fluence.transport.{ TransportSecurity, UPnP }
import fluence.transport.grpc.client.{ GrpcClient, GrpcClientConf }
import fluence.transport.grpc.server.{ GrpcServer, GrpcServerConf }
import monix.eval.Task
import monix.execution.Scheduler

import scala.concurrent.duration._
import scala.concurrent.{ ExecutionContext, Future }
import scala.language.higherKinds

class NodeComposer(
    keyPair: KeyPair,
    algo: SignAlgo,
    config: Config,
    contractCacheStore: KVStore[Task, Array[Byte], Array[Byte]],
    cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]] = JdkCryptoHasher.Sha256
) {

  private implicit val runFuture: Future ~> Task = new (Future ~> Task) {
    override def apply[A](fa: Future[A]): Task[A] = Task.deferFuture(fa)
  }

  private implicit val runTask: Task ~> Future = new (Task ~> Future) {
    // TODO: add logging
    override def apply[A](fa: Task[A]): Future[A] = fa.runAsync(Scheduler.global)
  }

  private implicit def runId[F[_]]: F ~> F = new (F ~> F) {
    override def apply[A](fa: F[A]): F[A] = fa
  }

  implicit val checker: SignatureChecker = algo.checker

  private implicit val kadCodec = fluence.kad.grpc.KademliaNodeCodec[Task]
  private implicit val contractCodec = fluence.dataset.grpc.BasicContractCodec.codec[Task]
  private val keyC = Key.bytesCodec[Task]

  import keyC.inverse

  private val signer = algo.signer(keyPair)

  private lazy val upnp = UPnP().unsafeRunSync()

  private lazy val serverBuilder =
    grpcConf.map(GrpcServer.builder(_, upnp))

  private lazy val grpcConf =
    GrpcServerConf.read[IO](config)

  private lazy val grpcClientConf =
    GrpcClientConf.read[IO](config)

  private def readKademliaConfig[F[_]](config: Config, path: String = "fluence.network.kademlia")(implicit F: ApplicativeError[F, Throwable]): F[KademliaConf] =
    F.catchNonFatal{
      import net.ceedubs.ficus.Ficus._
      import net.ceedubs.ficus.readers.ArbitraryTypeReader._
      config.as[KademliaConf](path)
    }

  lazy val contact: IO[Contact] =
    for {
      serverBuilder ← serverBuilder
      c ← Contact.buildOwn[IO](
        serverBuilder.address,
        serverBuilder.port,
        0l, // todo protocol version
        "", // todo git hash
        signer
      ).value.flatMap(MonadError[IO, Throwable].fromEither)
    } yield c

  lazy val services: IO[NodeServices[Task, BasicContract, Contact]] =
    for {
      k ← Key.fromKeyPair[IO](keyPair)
      conf ← grpcConf

      serverBuilder ← serverBuilder

      c ← contact

      kadConf ← readKademliaConfig[IO](config)

      clientConf ← grpcClientConf

      // TODO: externalize creation of signer/checker somehow

    } yield new NodeServices[Task, BasicContract, Contact] {

      private val client = {
        import monix.execution.Scheduler.Implicits.global // required for implicit Effect[Task]
        ClientComposer.grpc[Task](GrpcClient.builder(k, IO.pure(c.b64seed), clientConf))
      }

      override val key: Key = k

      override val signer: Signer = algo.signer(keyPair)

      override lazy val kademlia: Kademlia[Task, Contact] = KademliaMVar(
        k,
        Task.fromIO(contact).memoize,
        client.service[KademliaClient[Task]],
        kadConf,
        TransportSecurity.canBeSaved[Task](k, acceptLocal = conf.acceptLocal)
      )

      val contractsCacheStore: KVStore[Task, Key, ContractRecord[BasicContract]] = ContractsCacheStore(contractCacheStore)

      override lazy val contractsCache: ContractsCacheRpc[Task, BasicContract] =
        new ContractsCache[Task, BasicContract](
          nodeId = k,
          storage = contractsCacheStore,
          checker = checker,
          cacheTtl = 1.day
        )

      override lazy val contractAllocator: ContractAllocatorRpc[Task, BasicContract] =
        new ContractAllocator[Task, BasicContract](
          nodeId = k,
          storage = contractsCacheStore,
          createDataset = _ ⇒ Task.unit, // TODO: dataset creation
          checkAllocationPossible = _ ⇒ Task.unit, // TODO: check allocation possible
          checker = checker,
          signer = signer
        )

      override lazy val datasets: DatasetStorageRpc[Task] =
        new Datasets(
          config,
          cryptoHasher,

          // Return contract version, if current node participates in it
          contractsCacheStore.get(_)
            .map(c ⇒ Option(c.contract.executionState.version).filter(_ ⇒ c.contract.participants.contains(k))),

          // Update contract's version and merkle root, if newVersion = currentVersion+1
          (dsId, v, mr) ⇒ {
            // todo should be moved to separate class and write unit tests
            for {
              c ← contractsCacheStore.get(dsId)
              _ ← if (c.contract.executionState.version == v - 1) Task.unit
              else Task.raiseError(new IllegalStateException(s"Inconsistent state for contract $dsId, contract version=${c.contract.executionState.version}, asking for update to $v"))

              u = c.copy(
                contract = c.contract.copy(
                  executionState = BasicContract.ExecutionState(
                    version = v,
                    merkleRoot = mr
                  )
                ),
                lastUpdated = Instant.now()
              )
              _ ← contractsCacheStore.put(dsId, u)
            } yield () // TODO: schedule broadcasting the contract to kademlia

          }
        )
    }

  private val ec = ExecutionContext.fromExecutorService(Executors.newCachedThreadPool())

  // Add server (with kademlia inside), build
  lazy val server: IO[GrpcServer] =
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
        .add(DatasetStorageRpcGrpc.bindService(new DatasetStorageServer[Task](datasets), ec))
        .onNodeActivity(kademlia.update(_).toIO(Scheduler.global), clientConf, checker)
        .build
    }

}
