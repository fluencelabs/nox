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
import cats.effect.IO
import cats.syntax.flatMap._
import cats.syntax.show._
import com.typesafe.config.ConfigFactory
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.keypair.KeyPair
import fluence.crypto.{ FileKeyStorage, SignAlgo }
import fluence.dataset.BasicContract
import fluence.kad.Kademlia
import fluence.kad.protocol.Contact
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

  val algo: SignAlgo = Ecdsa.signAlgo

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
    kp ← prepareKeyPair().to[Task]

  } yield {

    sys.addShutdownHook {
      logger.warn("*** shutting down gRPC server since JVM is shutting down")
      //server.shutdown(5.seconds)
      logger.warn("*** server shut down")
    }

    logger.info("Server launched")
    //logger.info("Your contact is: " + contact.show)

    //logger.info("You may share this seed for others to join you: " + Console.MAGENTA + contact.b64seed + Console.RESET)
  }

  logger.info("Going to run Fluence Server...")

  serverKad
    .attempt
    .flatMap {
      case Left(t: Throwable) ⇒ Task.raiseError(t)
      case Left(t)            ⇒ Task.raiseError(new RuntimeException(t))
      case Right(_)           ⇒ Task.never
    }
    .toIO
    .unsafeRunSync()

  def prepareKeyPair(): IO[KeyPair] =
    getKeyPair[IO](config.getString("fluence.keyPath"))
  //  Read keypair, print "Loaded keypair, PK = ..."
  //  Or generate keypair, print

  def prepareServices(keyPair: KeyPair): IO[NodeServices[Task, BasicContract, Contact]] =
    for {
      contractCache ← RocksDbStore[IO](config.getString("fluence.contract.cacheDirName"), config)
      nodeComposer = new NodeComposer(keyPair, algo, config, contractCache)
      services ← nodeComposer.services
    } yield services

  def prepareContact() = ???

  def launchGrpcServer(services: NodeServices[Task, BasicContract, Contact]): IO[Kademlia[Task, Contact]] =
    for {
      _ ← IO.unit
      //server ← nodeComposer.server.toIO

      //_ ← //server.start().toIO
    } yield services.kademlia
  //  Print "Launching GRPC server on port XXX..."
  //  Print "Server launched"
  //  Print "Your contact is ... you may use it for other nodes or clients to join yours one"

  def joinNetwork(kademlia: Kademlia[Task, Contact]): IO[Unit] = ???
  // read the config
  //  Print "No seed nodes. Make others join you! Or correct config value ..."
  //
  //  Or print "Found N seeds, joining..."
  //
  //  Print "Joined N seed nodes, waiting for requests"
  //
  //  When requests are coming, tell about them: like "new node pinged us", "dataset created on the node", "dataset recovered from cache", etc.

}
