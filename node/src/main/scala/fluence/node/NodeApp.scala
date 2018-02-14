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
import java.util.concurrent.ConcurrentSkipListSet

import cats.MonadError
import cats.syntax.flatMap._
import cats.syntax.show._
import com.typesafe.config.ConfigFactory
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.keypair.KeyPair
import fluence.crypto.{ FileKeyStorage, SignAlgo }
import fluence.kad.Kademlia
import fluence.kad.protocol.{ Contact, Key }
import fluence.storage.rocksdb.RocksDbStore
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import slogging.{ LogLevel, LoggerConfig, PrintLoggerFactory }

import scala.concurrent.duration._
import scala.io.StdIn
import scala.language.higherKinds

object NodeApp extends App with slogging.LazyLogging {

  private val setForClose = new ConcurrentSkipListSet[AutoCloseable]()

  // Simply log everything to stdout
  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.INFO

  sys.ShutdownHookThread {
    logger.info(s"Invoke close() for all of $setForClose")
    setForClose.forEach(_.close())
  }

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
    keyStorage.getOrCreateKeyPair(algo.generateKeyPair[F]().value.flatMap(F.fromEither))
  }

  def cmd(s: String): Unit = logger.info(Console.BLUE + s + Console.RESET)

  val config = ConfigFactory.load()

  val gitHash = config.getString("fluence.gitHash")

  initDirectory(config.getString("fluence.directory"))

  logger.info(Console.CYAN + "Git Commit Hash: " + gitHash + Console.RESET)

  def handleCmds(kad: Kademlia[Task, Contact]): Task[Unit] = {
    cmd("j | l | s | q")
    val readLine = Task(StdIn.readLine())
    lazy val handle: Task[Unit] = readLine.flatMap {
      case "j" | "join" ⇒
        cmd("join seed?")
        readLine.flatMap { p ⇒
          Contact.readB64seed[Task](p, algo.checker).value.flatMap {
            case Left(err) ⇒
              logger.error("Can't read the seed", err)
              handle
            case Right(c) ⇒
              kad.join(Seq(c), 16).attempt.flatMap {
                case Right(_) ⇒
                  cmd("OK")
                  handle

                case Left(err) ⇒
                  logger.error("Can't join", err)
                  handle
              }
          }
        }

      case "l" | "lookup" ⇒

        kad.handleRPC
          .lookup(Key.XorDistanceMonoid.empty, 10)
          .map(_.map(_.show).mkString("\n"))
          .map(logger.info(_))
          .attempt
          .flatMap {
            case Right(_) ⇒
              logger.info("ok")
              handle
            case Left(e) ⇒
              logger.error("Can't lookup", e)
              handle
          }

      case "s" | "seed" ⇒
        kad.ownContact.map(_.contact).flatMap(_.b64seed[Task].value).map(e ⇒ e.map(cmd)).flatMap(_ ⇒ handle)

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
    contractCache ← RocksDbStore[Task](config.getString("fluence.contract.cacheDirName"), config)
    nodeComposer = new NodeComposer(kp, algo, config, contractCache)
    services ← nodeComposer.services
    server ← nodeComposer.server
    _ ← server.start()
    contact ← server.contact
  } yield {

    setForClose.add(contractCache)

    sys.addShutdownHook {
      logger.warn("*** shutting down gRPC server since JVM is shutting down")
      server.shutdown(5.seconds)
      logger.warn("*** server shut down")
    }

    logger.info("Server launched")
    logger.info("Your contact is: " + contact.show)
    contact.b64seed[cats.Id].value.map(s ⇒
      logger.info("You may share this seed for others to join you: " + Console.MAGENTA + s + Console.RESET)
    )

    services.kademlia
  }

  logger.info("Going to run Fluence Server...")

  serverKad.flatMap(handleCmds)
    .toIO
    .attempt
    .unsafeRunSync() match {
      case Left(err) ⇒
        err.printStackTrace()
        logger.error("Error", err)
      case Right(_) ⇒
        logger.info("Bye!")
    }

}
