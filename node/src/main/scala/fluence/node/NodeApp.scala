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
import fluence.crypto.FileKeyStorage
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.keypair.KeyPair
import fluence.info.NodeInfo
import fluence.kad.protocol.{ Contact, Key }
import monix.eval.{ Coeval, Task }
import monix.execution.Scheduler.Implicits.global
import org.slf4j.LoggerFactory

import scala.concurrent.Await
import scala.concurrent.duration.Duration
import scala.io.StdIn
import scala.language.higherKinds
import scala.util.{ Failure, Success }

object NodeApp extends App {

  def initDirectory(fluencePath: String): Unit = {
    val appDir = new File(fluencePath)
    if (!appDir.exists()) {
      appDir.getParentFile.mkdirs()
      appDir.mkdir()
    }
    ()
  }

  def getKeyPair[F[_]](keyPath: String)(implicit F: MonadError[F, Throwable]): F[KeyPair] = {
    val keyFile = new File(keyPath)
    val keyStorage = new FileKeyStorage[F](keyFile)
    keyStorage.getOrCreateKeyPair(Ecdsa.ecdsa_secp256k1_sha256[F].generateKeyPair())
  }

  def cmd(s: String): Unit = println(Console.BLUE + s + Console.RESET)

  val log = LoggerFactory.getLogger(getClass)

  val config = ConfigFactory.load()

  val gitHash = config.getString("fluence.gitHash")

  initDirectory(config.getString("fluence.directory"))

  println(Console.CYAN + "Git Commit Hash: " + gitHash + Console.RESET)

  val p = for {
    kp ← getKeyPair[Task](config.getString("fluence.keyPath"))
    nodeComposer = new NodeComposer(kp, () ⇒ Task.now(NodeInfo(gitHash)))
    services ← nodeComposer.services
    server ← nodeComposer.server
    kad = services.kademlia
    _ ← server.start()
    contact ← server.contact
  } yield {
    log.info("Server launched")
    println("Your contact is: " + contact.show)
    println("You may share this seed for others to join you: " + Console.MAGENTA + contact.b64seed + Console.RESET)

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

  println("going to run")

  Await.result(p.attempt.runAsync, Duration.Inf) match {
    case Left(err) ⇒ log.error("Error", err)
    case Right(_)  ⇒ println("Bye!")
  }

  println("result?")

}
