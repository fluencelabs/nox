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
import fluence.kad.grpc.KademliaGrpc
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.protocol.{ Contact, Key }
import fluence.transport.TransportSecurity
import fluence.transport.grpc.client.GrpcClient
import fluence.transport.grpc.server.{ GrpcServerConf, GrpcServer }
import monix.eval.{ Coeval, Task }
import org.slf4j.LoggerFactory
import cats.syntax.show._

import scala.concurrent.Future
import scala.concurrent.duration._
import scala.io.StdIn
import scala.util.{ Failure, Success }
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

  val serverBuilder = GrpcServer.builder

  // For demo purposes
  val rawKey = StdIn.readLine(Console.CYAN + "Who are you?\n> " + Console.RESET)

  val key = Key.sha1(rawKey.getBytes)

  // We have only Kademlia service client
  val client = GrpcClient.builder(key, serverBuilder.contact)
    .add(KademliaClient.register[Task]())
    .build

  // This is a server service, not a client
  val kad = new KademliaService(
    key,
    serverBuilder.contact,
    client.service[KademliaClient[Task]],
    KademliaConf.read(),
    TransportSecurity.canBeSaved[Task](key, acceptLocal = GrpcServerConf.read().acceptLocal)
  )

  // Add server (with kademlia inside), build
  val server = serverBuilder
    .add(KademliaGrpc.bindService(new KademliaServer[Task](kad.handleRPC), global))
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
