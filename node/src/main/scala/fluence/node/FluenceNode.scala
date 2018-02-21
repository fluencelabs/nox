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

import cats.{ Applicative, MonadError }
import cats.effect.IO
import cats.syntax.show._
import com.typesafe.config.{ Config, ConfigFactory }
import fluence.crypto.{ FileKeyStorage, SignAlgo }
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.hash.{ CryptoHasher, JdkCryptoHasher }
import fluence.crypto.keypair.KeyPair
import fluence.kad.protocol.{ Contact, KademliaRpc, Key, Node }
import monix.eval.Task
import cats.instances.list._
import fluence.client.SeedsConfig
import fluence.kad.Kademlia
import monix.execution.Scheduler

import scala.concurrent.duration._

trait FluenceNode {
  def config: Config

  def node: Task[Node[Contact]] = kademlia.ownContact

  def contact: Task[Contact] = node.map(_.contact)

  def kademlia: Kademlia[Task, Contact]

  def stop: IO[Unit]

  def restart: IO[FluenceNode]
}

object FluenceNode extends slogging.LazyLogging {

  /**
   * Launches a node with all available and enabled network interfaces.
   *
   * @param algo Algorithm to use for signatures
   * @param hasher Hasher, used in b-tree
   * @param config Configuration to read from
   * @return An IO that can be used to shut down the node
   */
  def startNode(
    algo: SignAlgo = Ecdsa.signAlgo,
    hasher: CryptoHasher[Array[Byte], Array[Byte]] = JdkCryptoHasher.Sha256,
    config: Config = ConfigFactory.load()
  ): IO[FluenceNode] =
    launchGrpc(algo, hasher, config)

  /**
   * Initiates a directory with all its parents
   *
   * @param path Directory path to create
   * @return Existing directory
   */
  private def initDirectory(path: String): IO[File] =
    IO {
      val appDir = new File(path)
      if (!appDir.exists()) {
        appDir.getParentFile.mkdirs()
        appDir.mkdir()
      }
      appDir
    }

  /**
   * Generates or loads keypair
   *
   * @param keyPath Path to store keys in
   * @param algo Sign algo
   * @return Keypair, either loaded or freshly generated
   */
  private def getKeyPair(keyPath: String, algo: SignAlgo): IO[KeyPair] = {
    val keyFile = new File(keyPath)
    val keyStorage = new FileKeyStorage[IO](keyFile)
    keyStorage.getOrCreateKeyPair(algo.generateKeyPair[IO]().value.flatMap(IO.fromEither))
  }

  /**
   * Launches GRPC node, using all the provided configs.
   * @return IO that will shutdown the node once ran
   */
  // todo write unit test, this method don't close resources correct when initialisation failed
  private def launchGrpc(algo: SignAlgo, hasher: CryptoHasher[Array[Byte], Array[Byte]], config: Config): IO[FluenceNode] = {
    import algo.checker
    for {
      _ ← initDirectory(config.getString("fluence.directory"))
      kp ← getKeyPair(config.getString("fluence.keyPath"), algo)
      key ← Key.fromKeyPair[IO](kp)

      builder ← NodeGrpc.grpcServerBuilder(config)

      contactConf ← ContactConf.read(config)
      contact ← Contact.buildOwn[IO](
        ip = contactConf.host,
        port = contactConf.port,
        protocolVersion = contactConf.protocolVersion,
        gitHash = contactConf.gitHash,
        signer = algo.signer(kp)
      ).value.flatMap(MonadError[IO, Throwable].fromEither)

      client ← NodeGrpc.grpcClient(key, contact, config)
      kadClient = client.service[KademliaRpc[Task, Contact]] _

      services ← NodeComposer.services(kp, contact, algo, hasher, kadClient, config, acceptLocal = true)

      server ← NodeGrpc.grpcServer(services, builder, config)

      _ ← server.start

      seedConfig ← SeedsConfig.read(config)
      seedContacts ← seedConfig.contacts

      _ ← if (seedContacts.nonEmpty) services.kademlia.join(seedContacts, 10).toIO(Scheduler.global) else IO{
        logger.info("You should add some seed node contacts to join. Take a look on reference.conf")
      }
    } yield {

      logger.info("Server launched")
      logger.info("Your contact is: " + contact.show)

      logger.info("You may share this seed for others to join you: " + Console.MAGENTA + contact.b64seed + Console.RESET)

      val _conf = config

      val node = new FluenceNode {
        override def config: Config = _conf

        override def kademlia: Kademlia[Task, Contact] = services.kademlia

        override def stop: IO[Unit] =
          Applicative[IO].map2(
            services.close,
            server.shutdown
          ){
              (_, _) ⇒ ()
            }

        override def restart: IO[FluenceNode] =
          stop.flatMap(_ ⇒ launchGrpc(algo, hasher, _conf))
      }

      sys.addShutdownHook {
        logger.warn("*** shutting down Fluence Node since JVM is shutting down")
        node.stop.unsafeRunTimed(5.seconds)
        logger.warn("*** node shut down")
      }

      node
    }
  }

}
