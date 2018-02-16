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

import cats.effect.IO
import cats.syntax.flatMap._
import cats.syntax.show._
import com.typesafe.config.ConfigFactory
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.hash.JdkCryptoHasher
import fluence.crypto.keypair.KeyPair
import fluence.crypto.{ FileKeyStorage, SignAlgo }
import fluence.kad.protocol.{ Contact, KademliaRpc, Key }
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

  // TODO: move it somewhere
  def initDirectory(fluencePath: String): IO[Unit] =
    IO {
      val appDir = new File(fluencePath)
      if (!appDir.exists()) {
        appDir.getParentFile.mkdirs()
        appDir.mkdir()
      }
    }

  val algo: SignAlgo = Ecdsa.signAlgo

  // TODO: move it somewhere
  private def getKeyPair(keyPath: String): IO[KeyPair] = {
    val keyFile = new File(keyPath)
    val keyStorage = new FileKeyStorage[IO](keyFile)
    keyStorage.getOrCreateKeyPair(algo.generateKeyPair[IO]().value.flatMap(IO.fromEither))
  }

  val config = ConfigFactory.load()

  val gitHash = config.getString("fluence.gitHash")

  logger.info(Console.CYAN + "Git Commit Hash: " + gitHash + Console.RESET)

  def launchGrpc(): IO[Unit] =
    for {
      _ ← initDirectory(config.getString("fluence.directory"))
      kp ← getKeyPair(config.getString("fluence.keyPath"))
      key ← Key.fromKeyPair[IO](kp)

      builder ← NodeGrpc.grpcServerBuilder(config)
      contact ← NodeGrpc.grpcContact(algo.signer(kp), builder)

      client ← NodeGrpc.grpcClient(key, contact, config)
      kadClient = client.service[KademliaRpc[Task, Contact]] _

      cacheStore ← RocksDbStore[IO](config.getString("fluence.contract.cacheDirName"), config)
      services ← NodeComposer.services(kp, contact, algo, JdkCryptoHasher.Sha256, cacheStore, kadClient, config, acceptLocal = true)

      server ← NodeGrpc.grpcServer(services, builder, config)

      _ ← server.start
    } yield {
      sys.addShutdownHook {
        logger.warn("*** shutting down gRPC server since JVM is shutting down")
        server.shutdown.unsafeRunTimed(5.seconds)
        logger.warn("*** server shut down")
      }

      logger.info("Server launched")
      logger.info("Your contact is: " + contact.show)

      logger.info("You may share this seed for others to join you: " + Console.MAGENTA + contact.b64seed + Console.RESET)

      // TODO: join network
    }

  logger.info("Going to run Fluence Server...")

  launchGrpc()
    .attempt
    .flatMap {
      case Left(t: Throwable) ⇒ IO.raiseError(t)
      case Left(t)            ⇒ IO.raiseError(new RuntimeException(t))
      case Right(_)           ⇒ Task.never.toIO
    }
    .unsafeRunSync()
}
