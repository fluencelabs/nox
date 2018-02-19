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

package fluence.client

import java.io.File

import cats.kernel.Monoid
import cats.~>
import com.typesafe.config.ConfigFactory
import fluence.crypto.KeyStore
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.hash.JdkCryptoHasher
import fluence.crypto.signature.SignatureChecker
import fluence.dataset.BasicContract
import fluence.dataset.client.Contracts
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsCacheRpc }
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.protocol.{ Contact, Key }
import fluence.kad.{ KademliaConf, KademliaMVar }
import fluence.transport.TransportSecurity
import fluence.transport.grpc.client.GrpcClient
import io.circe.parser.decode
import io.circe.syntax._
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import scodec.bits.{ Bases, ByteVector }
import scopt.Read.reads
import scopt.{ OptionParser, Read }
import slogging.{ LogLevel, LoggerConfig, PrintLoggerFactory }

import scala.concurrent.duration._
import scala.concurrent.{ Await, Future }
import scala.io.StdIn
import scala.language.higherKinds

case class CommandLineConfig(config: Option[File] = None, seed: Seq[String] = Seq.empty, keyStore: Option[KeyStore] = None)

object ClientApp extends App with slogging.LazyLogging {
  import KeyStore._

  val alphabet = Bases.Alphabets.Base64Url

  implicit val keyStoreRead: Read[KeyStore] = {
    reads { str ⇒
      val jsonStr = ByteVector.fromBase64(str, alphabet) match {
        case Some(bv) ⇒ new String(bv.toArray)
        case None ⇒
          throw new IllegalArgumentException("'" + str + "' is not a valid base64.")
      }
      decode[Option[KeyStore]](jsonStr) match {
        case Right(Some(ks)) ⇒ ks
        case Right(None)     ⇒ throw new IllegalArgumentException("'" + str + "' is not a valid key store.")
        case Left(err)       ⇒ throw new IllegalArgumentException("'" + str + "' is not a valid key store.", err)
      }
    }
  }

  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.INFO

  val parser = new OptionParser[CommandLineConfig]("scopt") {
    head("Fluence client")

    opt[File]('c', "config").valueName("<file>")
      .action((x, c) ⇒ c.copy(config = Some(x)))
      .text("Path to config file")

    opt[Seq[String]]('s', "seed").valueName("<seed1>,<seed2>...")
      .action((x, c) ⇒ c.copy(seed = x))
      .validate { seeds ⇒
        seeds.forall(s ⇒ ByteVector.fromBase64(s).isDefined) match {
          case true  ⇒ Right(())
          case false ⇒ Left("Some seeds is not valid")
        }
      }
      .text("Initial kademlia nodes contacts in base64 to connect with")

    opt[KeyStore]('k', "keystore").valueName("<keystore>")
      .action((x, c) ⇒ c.copy(keyStore = Some(x)))
      .text("Key pair in base64")

    help("help").text("Write help message")

    note("Arguments will override values in config file")
  }

  val algo = Ecdsa.signAlgo

  private implicit val checker: SignatureChecker = algo.checker

  private implicit def runTask[F[_]]: Task ~> Future = new (Task ~> Future) {
    override def apply[A](fa: Task[A]): Future[A] = fa.runAsync
  }

  private implicit def runFuture[F[_]]: Future ~> Task = new (Future ~> Task) {
    override def apply[A](fa: Future[A]): Task[A] = Task.fromFuture(fa)
  }

  private implicit def runId[F[_]]: F ~> F = new (F ~> F) {
    override def apply[A](fa: F[A]): F[A] = fa
  }

  // parser.parse returns Option[C]
  parser.parse(args, CommandLineConfig()) match {
    case Some(c) ⇒

      val config = c.config match {
        case Some(configFile) ⇒ ConfigFactory.parseFile(configFile)
        case None             ⇒ ConfigFactory.load()
      }

      val seedB64 = config.getString("fluence.seed")

      val seed = Await.result(Contact.readB64seed[Task](seedB64, checker).value.runAsync, 4.second).right.get

      val keyPair = c.keyStore.map(_.keyPair)
        .getOrElse(Await.result(algo.generateKeyPair[Task]().value.runAsync, 5.seconds).right.get)

      logger.info("Your keypair is:")
      logger.info(ByteVector(KeyStore(keyPair).asJson.noSpaces.getBytes()).toBase64(alphabet))
      logger.info("Store it and use it as auth.")

      val client = ClientComposer.grpc[Task](GrpcClient.builder)

      val conf = KademliaConf(100, 10, 2, 5.seconds)
      val clKey = Monoid.empty[Key]
      val check = TransportSecurity.canBeSaved[Task](clKey, acceptLocal = true)
      val kademliaRpc = client.service[KademliaClient[Task]] _
      val kademliaClient = KademliaMVar.client(kademliaRpc, conf, check)

      logger.info("Connecting to seed node.")

      Await.ready(kademliaClient.join(Seq(seed), 2).runAsync, 5.second)

      val contracts = new Contracts[Task, BasicContract, Contact](
        maxFindRequests = 10,
        maxAllocateRequests = _ ⇒ 20,
        checker = algo.checker,
        kademlia = kademliaClient,
        cacheRpc = contact ⇒ client.service[ContractsCacheRpc[Task, BasicContract]](contact),
        allocatorRpc = contact ⇒ client.service[ContractAllocatorRpc[Task, BasicContract]](contact)
      )

      logger.info("Creating fluence client.")

      val fluenceClient = FluenceClient(kademliaClient, contracts, client.service[DatasetStorageRpc[Task]], algo, JdkCryptoHasher.Sha256)

      while (true) {
        val args = StdIn.readLine()
        if (args.nonEmpty) {
          handleCommands(args, fluenceClient, AuthorizedClient(keyPair))
        }
      }

    case None ⇒
    // arguments are bad, error message will have been displayed
  }

  def handleCommands(line: String, fluenceClient: FluenceClient, ac: AuthorizedClient): Unit = {
    val commandOp = CommandParser.parseCommand(line)
    commandOp match {
      case Some(Exit) ⇒
        logger.info("Exiting from fluence network.")
        System.exit(0)
      case Some(Put(k, v)) ⇒
        val t = for {
          ds ← fluenceClient.getOrCreateDataset(ac)
          _ ← ds.put(k, v)
        } yield ()

        Await.ready(t.runAsync, 5.seconds)
        logger.info("Success.")

      case Some(Get(k)) ⇒
        val t = for {
          ds ← fluenceClient.getOrCreateDataset(ac)
          res ← ds.get(k)
        } yield res

        val res = Await.result(t.runAsync, 5.seconds) match {
          case Some(r) ⇒ r
          case None    ⇒ "None"
        }
        logger.info("Result: " + res)
      case _ ⇒ logger.info("something wrong")
    }
  }

}
