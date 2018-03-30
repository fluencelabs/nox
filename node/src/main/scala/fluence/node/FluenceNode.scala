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
import java.net.InetAddress
import java.time.Clock

import cats.effect.IO
import cats.instances.list._
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.show._
import cats.{Applicative, MonadError}
import com.typesafe.config.{Config, ConfigFactory, ConfigRenderOptions}
import fluence.client.core.config.{KeyPairConfig, SeedsConfig}
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.hash.{CryptoHasher, JdkCryptoHasher}
import fluence.crypto.{FileKeyStorage, SignAlgo}
import fluence.kad.Kademlia
import fluence.kad.protocol.{Contact, Key, Node}
import fluence.node.core.NodeComposer
import fluence.node.core.config.{ContactConf, UPnPConf}
import fluence.node.grpc.NodeGrpc
import fluence.transport.UPnP
import fluence.transport.grpc.server.GrpcServerConf
import monix.eval.Task
import monix.execution.Scheduler

import scala.concurrent.duration._
import scala.language.higherKinds

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
  ): IO[FluenceNode] = {
    logger.debug(
      "Node config is :" +
        config.getConfig("fluence").root().render(ConfigRenderOptions.defaults().setOriginComments(false))
    )
    launchGrpc(algo, hasher, config)
  }

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

  private def launchUPnP(config: Config, contactConf: ContactConf, grpc: GrpcServerConf): IO[(ContactConf, IO[Unit])] =
    UPnPConf.read(config).flatMap {
      case u if !u.isEnabled ⇒ IO.pure(contactConf -> IO.unit)
      case u ⇒
        UPnP().flatMap { upnp ⇒
          // Provide upnp-discovered external address to contact conf
          val contact = contactConf.copy(host = Some(upnp.externalAddress))

          // Forward grpc port
          u.grpc.fold(IO.pure(contact -> IO.unit)) { grpcExternalPort ⇒
            upnp
              .addPort(grpcExternalPort, grpc.port)
              .map(_ ⇒ contact.copy(grpcPort = Some(grpcExternalPort)) -> upnp.deletePort(grpcExternalPort))
          }
        }
    }

  /**
   * Launches GRPC node, using all the provided configs.
   * @return IO that will shutdown the node once ran
   */
  // todo write unit test, this method don't close resources correctly when initialisation failed
  private def launchGrpc(
    algo: SignAlgo,
    hasher: CryptoHasher[Array[Byte], Array[Byte]],
    config: Config,
    clock: Clock = Clock.systemUTC()
  ): IO[FluenceNode] = {
    import algo.checker
    for {
      _ ← initDirectory(config.getString("fluence.directory")) // TODO config
      kpConf ← KeyPairConfig.read(config)
      kp ← FileKeyStorage.getKeyPair[IO](kpConf.keyPath, algo)
      key ← Key.fromKeyPair[IO](kp)

      grpcServerConf ← NodeGrpc.grpcServerConf(config)
      builder ← NodeGrpc.grpcServerBuilder(grpcServerConf)

      contactConf ← ContactConf.read(config)

      upnpContactStop ← launchUPnP(config, contactConf, grpcServerConf)

      (upnpContact, upnpShutdown) = upnpContactStop

      contact ← Contact
        .buildOwn[IO](
          addr = upnpContact.host.getOrElse(InetAddress.getLocalHost).getHostName,
          port = upnpContact.grpcPort.getOrElse(builder.port),
          protocolVersion = upnpContact.protocolVersion,
          gitHash = upnpContact.gitHash,
          signer = algo.signer(kp)
        )
        .value
        .flatMap(MonadError[IO, Throwable].fromEither)
        .onFail(upnpShutdown)

      client ← NodeGrpc.grpcClient(key, contact, config)
      kadClient = client(_: Contact).kademlia

      services ← NodeComposer
        .services(kp, contact, algo, hasher, kadClient, config, acceptLocal = true, clock)(Scheduler.global) // TODO: it should be custom
        .onFail(upnpShutdown)
      closeUpNpAndServices = upnpShutdown.flatMap(_ ⇒ services.close)

      server ← NodeGrpc.grpcServer(services, builder, config).onFail(closeUpNpAndServices)

      _ ← server.start.onFail(closeUpNpAndServices)
      http4s ← IO(HttpProxyHttp4s.builder(server).unsafeRunSync())
      closeAll = closeUpNpAndServices.flatMap(_ ⇒ server.shutdown).flatMap(_ ⇒ http4s.shutdown)

      seedConfig ← SeedsConfig.read(config).onFail(closeAll)
      seedContacts ← seedConfig.contacts.onFail(closeAll)

      _ ← if (seedContacts.nonEmpty) services.kademlia.join(seedContacts, 10).toIO(Scheduler.global)
      else
        IO {
          logger.info("You should add some seed node contacts to join. Take a look on reference.conf")
        }.onFail(closeAll)
    } yield {

      logger.info("Server launched")
      logger.info("Your contact is: " + contact.show)

      logger.info(
        "You may share this seed for others to join you: " + Console.MAGENTA + contact.b64seed + Console.RESET
      )

      val _conf = config

      val node = new FluenceNode {
        override def config: Config = _conf

        override def kademlia: Kademlia[Task, Contact] = services.kademlia

        override def stop: IO[Unit] =
          Applicative[IO].map3(
            server.shutdown,
            services.close,
            upnpShutdown
          ) { (_, _, _) ⇒
            ()
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

  implicit class ResourceCloser[F[_], T](origin: F[T])(implicit val ME: MonadError[F, Throwable]) {

    /** Invoke ''closeFn'' and raise error if ''origin'' failed */
    def onFail(closeFn: F[Unit]): F[T] = {
      origin.attempt.flatMap {
        case Left(err) ⇒
          closeFn.flatMap(_ ⇒ ME.raiseError(err))
        case Right(s) ⇒ ME.pure(s)
      }
    }
  }

}
