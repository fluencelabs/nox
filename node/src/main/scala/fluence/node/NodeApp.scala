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
import cats.syntax.flatMap._
import cats.syntax.show._
import com.typesafe.config.ConfigFactory
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.keypair.KeyPair
import fluence.crypto.{ FileKeyStorage, SignAlgo }
import fluence.storage.rocksdb.RocksDbStore
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import slogging.{ LogLevel, LoggerConfig, PrintLoggerFactory }

import scala.concurrent.duration._
import scala.language.higherKinds

object NodeApp extends App with slogging.LazyLogging {

  // Simply log everything to stdout
  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.INFO

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

  val serverKad = for {
    kp ← getKeyPair[Task](config.getString("fluence.keyPath"))
    contractCache ← RocksDbStore[Task](config.getString("fluence.contract.cacheDirName"), config)
    nodeComposer = new NodeComposer(kp, algo, config, contractCache)
    server ← nodeComposer.server
    _ ← server.start()
    contact ← server.contact
  } yield {

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

  }

  logger.info("Going to run Fluence Server...")

  serverKad.flatMap {
    case Left(t: Throwable) ⇒ Task.raiseError(t)
    case Left(t)            ⇒ Task.raiseError(new RuntimeException(t))
    case Right(_)           ⇒ Task.never
  }
    .toIO
    .attempt
    .unsafeRunSync() match {
      case Left(err) ⇒
        err.printStackTrace()
        logger.error("Error", err)
      case Right(_) ⇒
        logger.info("Bye!")
    }

  //Launch the node
  //
  //  Node prints "Fluence Node starting"
  //
  //  Read keypair, print "Loaded keypair, PK = ..."
  //
  //  Or generate keypair, print
  //
  //  Generate kademlia ID, print "Node ID = ..."
  //
  //  Print "Launching GRPC server on port XXX..."
  //
  //  Print "Server launched"
  //
  //  Print "Your contact is ... you may use it for other nodes or clients to join yours one"
  //
  //  Print "No seed nodes. Make others join you! Or correct config value ..."
  //
  //  Or print "Found N seeds, joining..."
  //
  //  Print "Joined N seed nodes, waiting for requests"
  //
  //  When requests are coming, tell about them: like "new node pinged us", "dataset created on the node", "dataset recovered from cache", etc.

}
