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

import java.io.File

import cats.MonadError
import cats.syntax.show._
import com.typesafe.config.ConfigFactory
import fluence.crypto.{ FileKeyStorage, SignAlgo }
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.keypair.KeyPair
import fluence.info.NodeInfo
import fluence.kad.Kademlia
import fluence.kad.protocol.{ Contact, Key }
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import org.slf4j.LoggerFactory

import scala.io.StdIn
import scala.concurrent.duration._
import scala.language.higherKinds

object NodeApp extends App {

  def initDirectory(fluencePath: String): Unit = {
    val appDir = new File(fluencePath)
    if (!appDir.exists()) {
      appDir.getParentFile.mkdirs()
      appDir.mkdir()
    }
    ()
  }

  val algo: SignAlgo = new SignAlgo(Ecdsa.ecdsa_secp256k1_sha256)

  def getKeyPair[F[_]](keyPath: String)(implicit F: MonadError[F, Throwable]): F[KeyPair] = {
    val keyFile = new File(keyPath)
    val keyStorage = new FileKeyStorage[F](keyFile)
    keyStorage.getOrCreateKeyPair(algo.generateKeyPair[F]())
  }

  def cmd(s: String): Unit = println(Console.BLUE + s + Console.RESET)

  val log = LoggerFactory.getLogger(getClass)

  val config = ConfigFactory.load()

  val gitHash = config.getString("fluence.gitHash")

  initDirectory(config.getString("fluence.directory"))

  println(Console.CYAN + "Git Commit Hash: " + gitHash + Console.RESET)

  def handleCmds(kad: Kademlia[Task, Contact]): Task[Unit] = {
    val readLine = Task(StdIn.readLine())
    lazy val handle: Task[Unit] = readLine.flatMap {
      case "j" | "join" ⇒
        cmd("join seed?")
        readLine.flatMap { p ⇒
          Task.fromEval(Contact.readB64seed(p)).attempt.flatMap {
            case Left(err) ⇒
              log.error("Can't read the seed", err)
              handle
            case Right(c) ⇒
              kad.join(Seq(c), 16).attempt.flatMap {
                case Right(_) ⇒
                  cmd("OK")
                  handle

                case Left(err) ⇒
                  log.error("Can't join", err)
                  handle
              }
          }
        }

      case "l" | "lookup" ⇒

        kad.handleRPC
          .lookup(Key.XorDistanceMonoid.empty, 10)
          .map(_.map(_.show).mkString("\n"))
          .map(println)
          .attempt
          .flatMap {
            case Right(_) ⇒
              println("ok")
              handle
            case Left(e) ⇒
              log.error("Can't lookup", e)
              handle
          }

      case "s" | "seed" ⇒
        kad.ownContact.map(_.contact).map(_.b64seed).map(cmd).flatMap(_ ⇒ handle)

      case "q" | "quit" | "x" | "exit" ⇒
        cmd("exit")
        Task.unit

      case _ ⇒
        handle
    }.asyncBoundary

    handle
  }

  val serverKad = for {
    kp ← getKeyPair[Task](config.getString("fluence.keyPath"))
    nodeComposer = new NodeComposer(kp, algo, config, () ⇒ Task.now(NodeInfo(gitHash)))
    services ← nodeComposer.services
    server ← nodeComposer.server
    _ ← server.start()
    contact ← server.contact
  } yield {

    sys.addShutdownHook {
      System.err.println("*** shutting down gRPC server since JVM is shutting down")
      server.shutdown(5.seconds)
      System.err.println("*** server shut down")
    }

    log.info("Server launched")
    println("Your contact is: " + contact.show)
    println("You may share this seed for others to join you: " + Console.MAGENTA + contact.b64seed + Console.RESET)

    services.kademlia
  }

  println("Going to run Fluence Server...")

  serverKad.flatMap(handleCmds)
    .toIO
    .attempt
    .unsafeRunSync() match {
      case Left(err) ⇒
        err.printStackTrace()
        log.error("Error", err)
      case Right(_) ⇒ println("Bye!")
    }

}
