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
import cats.~>
import com.typesafe.config.Config
import fluence.client.core.ClientServices
import fluence.client.grpc.ClientGrpcServices
import fluence.contract.BasicContract
import fluence.crypto.signature.SignatureChecker
import fluence.contract.grpc.server.{ ContractAllocatorServer, ContractsCacheServer }
import fluence.dataset.grpc.{ DatasetStorageRpcGrpc, DatasetStorageServer }
import fluence.contract.grpc.{ ContractAllocatorGrpc, ContractsCacheGrpc }
import fluence.kad.grpc.KademliaGrpc
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.protocol.{ Contact, Key }
import fluence.node.NodeComposer.Services
import fluence.transport.grpc.GrpcConf
import fluence.transport.grpc.client.GrpcClient
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

  def grpcClient(key: Key, contact: Contact, config: Config)(
      implicit checker: SignatureChecker
  ): IO[Contact ⇒ ClientServices[Task, BasicContract, Contact]] =
    for {
      clientConf ← GrpcConf.read[IO](config)
      client = {
        // TODO: check if it's optimal
        implicit val ec: Scheduler = Scheduler(Executors.newCachedThreadPool()) // required for implicit Effect[Task]
        ClientGrpcServices.build[Task](GrpcClient.builder(key, IO.pure(contact.b64seed), clientConf))
      }
    } yield client

  // Add server (with kademlia inside), build
  def grpcServer(
      services: Services,
      serverBuilder: GrpcServer.Builder,
      config: Config
  ): IO[GrpcServer] =
    for {
      clientConf ← GrpcConf.read[IO](config)
    } yield {
      import services._

      val _signAlgo = services.signAlgo
      import _signAlgo.checker

      // TODO: check if it's optimal
      implicit val ec: Scheduler = Scheduler(Executors.newCachedThreadPool())

      import fluence.contract.grpc.BasicContractCodec.{ codec ⇒ contractCodec }
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

  def grpcServerConf(config: Config): IO[GrpcServerConf] =
    for {
      serverConfOpt ← GrpcConf.read[IO](config).map(_.server)
      serverConf ← serverConfOpt match {
        case Some(sc) ⇒ IO.pure(sc)
        case None     ⇒ IO.raiseError(new IllegalStateException("fluence.grpc.server config is not defined"))
      }
    } yield serverConf

  def grpcServerBuilder(serverConf: GrpcServerConf): IO[GrpcServer.Builder] =
    IO.pure(GrpcServer.builder(serverConf))

}
