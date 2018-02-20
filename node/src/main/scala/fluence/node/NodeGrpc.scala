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

import cats.effect.IO
import cats.{ MonadError, ~> }
import com.typesafe.config.Config
import fluence.client.ClientComposer
import fluence.crypto.signature.{ SignatureChecker, Signer }
import fluence.dataset.BasicContract
import fluence.dataset.grpc.server.{ ContractAllocatorServer, ContractsCacheServer }
import fluence.dataset.grpc.storage.DatasetStorageRpcGrpc
import fluence.dataset.grpc.{ ContractAllocatorGrpc, ContractsCacheGrpc, DatasetStorageServer }
import fluence.kad.grpc.KademliaGrpc
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.protocol.{ Contact, Key }
import fluence.node.NodeComposer.Services
import fluence.transport.UPnP
import fluence.transport.grpc.client.{ GrpcClient, GrpcClientConf }
import fluence.transport.grpc.server.{ GrpcServer, GrpcServerConf }
import monix.eval.Task
import monix.execution.Scheduler

import scala.concurrent.Future
import scala.language.higherKinds

object NodeGrpc {

  private implicit val runTask: Task ~> Future = new (Task ~> Future) {
    // TODO: add logging
    override def apply[A](fa: Task[A]): Future[A] = fa.runAsync(Scheduler.global)
  }

  // TODO: remove it
  private implicit def runId[F[_]]: F ~> F = new (F ~> F) {
    override def apply[A](fa: F[A]): F[A] = fa
  }

  def grpcContact(signer: Signer, serverBuilder: GrpcServer.Builder): IO[Contact] =
    Contact.buildOwn[IO](
      serverBuilder.address,
      serverBuilder.port,
      0l, // todo protocol version
      "", // todo git hash
      signer
    ).value.flatMap(MonadError[IO, Throwable].fromEither)

  def grpcClient(key: Key, contact: Contact, config: Config)(implicit checker: SignatureChecker) =
    for {
      clientConf ← GrpcClientConf.read[IO](config)
      client = {
        // TODO: check if it's optimal
        implicit val ec: Scheduler = Scheduler(Executors.newCachedThreadPool()) // required for implicit Effect[Task]
        ClientComposer.grpc[Task](GrpcClient.builder(key, IO.pure(contact.b64seed), clientConf))
      }
    } yield client

  // Add server (with kademlia inside), build
  def grpcServer(
    services: Services,
    serverBuilder: GrpcServer.Builder,
    config: Config
  ): IO[GrpcServer] =
    for {
      clientConf ← GrpcClientConf.read[IO](config)
    } yield {
      import services._

      val _signAlgo = services.signAlgo
      import _signAlgo.checker

      // TODO: check if it's optimal
      implicit val ec: Scheduler = Scheduler(Executors.newCachedThreadPool())

      import fluence.dataset.grpc.BasicContractCodec.{ codec ⇒ contractCodec }
      import fluence.kad.grpc.KademliaNodeCodec.{ codec ⇒ nodeCodec }
      val keyC = Key.bytesCodec[Task]
      import keyC.inverse

      serverBuilder
        .add(KademliaGrpc.bindService(new KademliaServer[Task](kademlia.handleRPC), ec))
        .add(ContractsCacheGrpc.bindService(new ContractsCacheServer[Task, BasicContract](contractsCache), ec))
        .add(ContractAllocatorGrpc.bindService(new ContractAllocatorServer[Task, BasicContract](contractAllocator), ec))
        .add(DatasetStorageRpcGrpc.bindService(new DatasetStorageServer[Task](datasets), ec))
        .onNodeActivity(kademlia.update(_).toIO(Scheduler.global), clientConf)
        .build
    }

  def grpcServerBuilder(config: Config): IO[GrpcServer.Builder] =
    for {
      serverConf ← GrpcServerConf.read[IO](config)
    } yield GrpcServer.builder(serverConf, UPnP().unsafeRunSync())

}
