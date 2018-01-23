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

import cats.~>
import cats.instances.try_._
import fluence.kad.grpc.KademliaGrpc
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.protocol.{ Contact, Key }
import fluence.transport.TransportSecurity
import fluence.transport.grpc.client.GrpcClient
import fluence.transport.grpc.server.{ GrpcServer, GrpcServerConf }
import monix.eval.{ Coeval, Task }
import org.slf4j.LoggerFactory
import cats.syntax.show._
import fluence.client.ClientComposer
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{ SignatureChecker, Signer }
import fluence.dataset.BasicContract
import fluence.dataset.grpc.{ ContractAllocatorGrpc, ContractsCacheGrpc, DatasetContractsApiGrpc }
import fluence.dataset.grpc.server.{ ContractAllocatorServer, ContractsApiServer, ContractsCacheServer }
import fluence.dataset.node.Contracts
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsCacheRpc }
import fluence.storage.TrieMapKVStore

import scala.concurrent.Future
import scala.concurrent.duration._
import scala.io.StdIn
import scala.util.{ Failure, Success, Try }
import monix.execution.Scheduler.Implicits.global

object NodeApp extends App {

  val log = LoggerFactory.getLogger(getClass)

  private implicit val runFuture = new (Future ~> Task) {
    override def apply[A](fa: Future[A]): Task[A] = Task.deferFuture(fa)
  }

  private implicit val runTask = new (Task ~> Future) {
    // TODO: add logging
    override def apply[A](fa: Task[A]): Future[A] = fa.runAsync
  }

  implicit val kadCodec = fluence.kad.grpc.KademliaNodeCodec[Task]
  implicit val contractCodec = fluence.dataset.grpc.BasicContractCodec.codec[Task]
  val keyC = Key.bytesCodec[Task]
  import keyC.inverse

  val serverBuilder = GrpcServer.builder

  // For demo purposes
  val keySeed = StdIn.readLine(Console.CYAN + "Who are you?\n> " + Console.RESET)
  val keyPair = KeyPair.fromBytes(keySeed.getBytes(), keySeed.getBytes())

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

  sys.addShutdownHook {
    log.warn("*** shutting down gRPC server since JVM is shutting down")
    server.shutdown(10.seconds)
    log.warn("*** server shut down")
  }

  server.start().attempt.runAsync.foreach {
    case Left(err) ⇒
      log.error("Can't launch server", err)

    case Right(_) ⇒
      log.info("Server launched")

      server.contact.foreach{ contact ⇒
        println("Your contact is: " + contact.show)
        println("You may share this seed for others to join you: " + Console.MAGENTA + contact.b64seed + Console.RESET)
      }
  }

  def cmd(s: String): Unit = println(Console.BLUE + s + Console.RESET)

  while (true) {

    cmd("join(j) / lookup(l)")

    StdIn.readLine() match {
      case "j" | "join" ⇒
        cmd("join seed?")
        val p = StdIn.readLine()
        Coeval.fromEval(
          Contact.readB64seed(p)
        ).memoize.attempt.value match {
            case Left(err) ⇒
              log.error("Can't read the seed", err)

            case Right(c) ⇒
              kad.join(Seq(c), 16).runAsync.onComplete {
                case Success(_) ⇒ cmd("ok")
                case Failure(e) ⇒
                  log.error("Can't join", e)
              }
          }

      case "l" | "lookup" ⇒
        cmd("lookup myself")
        kad.handleRPC
          .lookup(Key.XorDistanceMonoid.empty, 10)
          .map(_.map(_.show).mkString("\n"))
          .map(println)
          .runAsync.onComplete {
            case Success(_) ⇒ println("ok")
            case Failure(e) ⇒
              log.error("Can't lookup", e)
          }

      case "s" | "seed" ⇒
        server.contact.map(_.b64seed).runAsync.foreach(cmd)

      case "q" | "quit" | "x" | "exit" ⇒
        cmd("exit")
        System.exit(0)

      case _ ⇒
    }
  }
}
