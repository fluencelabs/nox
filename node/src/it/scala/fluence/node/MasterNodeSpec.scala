/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.node

import java.nio.file.{Files, Paths}
import java.util.Base64

import cats.{Apply, Traverse}
import cats.effect._
import cats.syntax.apply._
import cats.syntax.functor._
import cats.syntax.traverse._
import cats.instances.list._
import com.softwaremill.sttp._
import com.softwaremill.sttp.circe.asJson
import fluence.crypto.eddsa.Ed25519
import fluence.effects.docker.DockerIO
import fluence.effects.ethclient.EthClient
import fluence.effects.kvstore.MVarKVStore
import fluence.effects.sttp.{SttpEffect, SttpStreamEffect}
import fluence.effects.testkit.Timed
import fluence.effects.{Backoff, EffectError}
import fluence.kad.Kademlia
import fluence.kad.conf.{AdvertizeConf, JoinConf, KademliaConfig, RoutingConf}
import fluence.kad.contact.UriContact
import fluence.kad.http.KademliaHttpNode
import fluence.kad.protocol.Key
import fluence.log.Log.Aux
import fluence.log.appender.PrintlnLogAppender
import fluence.log.{Log, LogFactory}
import fluence.node.config.{FluenceContractConfig, MasterConfig}
import fluence.node.eth.FluenceContractTestOps._
import fluence.node.eth.{FluenceContract, NodeEthState}
import fluence.node.status.{MasterStatus, StatusAggregator}
import fluence.node.workers.WorkersPorts
import fluence.node.workers.tendermint.ValidatorPublicKey
import fluence.worker.WorkerStage.{Destroyed, FullyAllocated}
import fluence.worker.WorkersPool
import org.scalatest.{Timer => _, _}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._
import scala.language.higherKinds

class MasterNodeSpec
    extends WordSpec with Matchers with BeforeAndAfterAll with OptionValues with Timed with GanacheSetup {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  implicit private val logFactory: LogFactory.Aux[IO, PrintlnLogAppender[IO]] = LogFactory.forPrintln[IO](Log.Trace)

  implicit private val backoff: Backoff[EffectError] = Backoff.default

  override protected def beforeAll(): Unit = {
    implicit val log: Aux[IO, PrintlnLogAppender[IO]] = logFactory.init("before").unsafeRunSync()
    wireupContract()
  }

  override protected def afterAll(): Unit = {
    implicit val log: Aux[IO, PrintlnLogAppender[IO]] = logFactory.init("after").unsafeRunSync()
    killGanache()
  }

  def getStatus(statusPort: Short)(implicit sttpBackend: SttpEffect[IO]): IO[MasterStatus] = {
    import MasterStatus._
    (for {
      resp <- sttp.response(asJson[MasterStatus]).get(uri"http://127.0.0.1:$statusPort/status").send()
    } yield resp.unsafeBody).value.flatMap(r => IO.fromEither(r.flatMap(_.left.map(_.error))))
  }

  def getEthState(statusPort: Short)(implicit sttpBackend: SttpEffect[IO]): IO[NodeEthState] = {
    import NodeEthState._
    (for {
      resp <- sttp.response(asJson[NodeEthState]).get(uri"http://127.0.0.1:$statusPort/status/eth").send()
    } yield resp.unsafeBody).value.flatMap(r => IO.fromEither(r.flatMap(_.left.map(_.error))))
  }

  private val masterConfF = MasterConfig
    .load()
    .map(_.copy(rootPath = Files.createTempDirectory("masternodespec").toString))

  private val nodeAddress = "vAs+M0nQVqntR6jjPqTsHpJ4bsswA3ohx05yorqveyc="

  private def nodeResource(port: Short = 5789, seeds: Seq[String] = Seq.empty)(
    implicit log: Log[IO]
  ): Resource[
    IO,
    (SttpStreamEffect[IO],
     MasterNode[IO, EmbeddedWorkerPool.Companions[IO]],
     ValidatorPublicKey,
     Kademlia[IO, UriContact])
  ] =
    for {
      implicit0(sttpB: SttpStreamEffect[IO]) ← SttpEffect.streamResource[IO]

      nodeCodec = new UriContact.NodeCodec(Key.fromPublicKey)

      masterConf <- Resource.liftF(masterConfF)
      implicit0(dockerIO: DockerIO[IO]) <- DockerIO.make[IO]()

      portsStore ← MVarKVStore.make[IO, Long, Short]()
      ports ← WorkersPorts.make[IO](masterConf.ports.minPort, masterConf.ports.maxPort, portsStore)

      kad ← KademliaHttpNode.make[IO](
        KademliaConfig(
          RoutingConf(1, 1, 4, 5.seconds),
          AdvertizeConf("127.0.0.1", port),
          JoinConf(seeds, 4)
        ),
        Ed25519.signAlgo,
        Ed25519.signAlgo.generateKeyPair.unsafe(Some(ByteVector.fromShort(port).toArray)),
        Paths.get(masterConf.rootPath),
        nodeCodec
      )

      publicKey = ValidatorPublicKey("", Base64.getEncoder.encodeToString(Array.fill(32)(5)))

      pool ← EmbeddedWorkerPool.embedded[IO](ports)

      node ← MasterNode.make(masterConf, publicKey, pool)

      agg ← StatusAggregator.make[IO](masterConf, pool, node.nodeEth)
      _ ← TestHttpServer.make("127.0.0.1", port, agg, kad.http)
      _ <- Log.resource[IO].info(s"Started MasterHttp")
    } yield (sttpB, node, publicKey, kad.kademlia)

  def fiberResource[F[_]: Concurrent: Log, A](f: F[A]): Resource[F, Unit] =
    Resource.make(Concurrent[F].start(f))(_.cancel).void

  def runningNode(port: Short = 5789, seeds: Seq[String] = Seq.empty)(
    implicit log: Log[IO]
  ): Resource[IO,
              (SttpStreamEffect[IO],
               MasterNode[IO, EmbeddedWorkerPool.Companions[IO]],
               ValidatorPublicKey,
               Kademlia[IO, UriContact])] =
    nodeResource(port, seeds).flatMap {
      case res @ (_, n, _, _) ⇒ fiberResource(n.run).as(res)
    }

  private def checkDestroyed[A, B <: shapeless.HList](pool: WorkersPool[IO, A, B], appId: Long): IO[Unit] = {
    pool
      .get(appId)
      .value
      .flatMap {
        case Some(w) =>
          w.stage.map {
            case Destroyed => true
            case _         => false
          }
        case None => IO(false)
      }
      .map(_ shouldBe true)
  }

  private def checkAlive[A, B <: shapeless.HList](pool: WorkersPool[IO, A, B], number: Int): IO[Unit] = {
    pool
      .listAll()
      .map(l => l.map(_.stage))
      .flatMap(l => Traverse[List].traverse(l)(identity))
      .map {
        _.filter {
          case FullyAllocated => true
          case _              => false
        }
      }
      .map(_.size shouldBe number)
  }

  "MasterNode" should {
    "provide status" in {
      implicit val log: Log[IO] = logFactory.init("spec", "status").unsafeRunSync()

      runningNode().use {
        case (sttpB, _, _, _) ⇒
          implicit val s: SttpStreamEffect[IO] = sttpB
          log.debug("Going to run the node").unsafeRunSync()

          eventually[IO](
            for {
              status <- getStatus(5789)
              _ = (Math.abs(status.uptime) > 0) shouldBe true
            } yield (),
            100.millis,
            15.seconds
          )

      }.unsafeRunSync()

    }

    "run and delete apps properly" in {
      implicit val log: Log[IO] = logFactory.init("spec", "apps").unsafeRunSync()

      (runningNode(), EthClient.make[IO]()).tupled.use {
        case ((sttpB, node, publicKey, _), ethClient) ⇒
          implicit val s = sttpB

          val contractAddress = "0x9995882876ae612bfd829498ccd73dd962ec950a"
          val owner = "0x4180FC65D613bA7E1a385181a219F1DBfE7Bf11d"

          val contractConfig = FluenceContractConfig(owner, contractAddress)

          val contract = FluenceContract(ethClient, contractConfig)

          for {
            masterConf <- masterConfF
            _ ← contract
              .addNode[IO](
                publicKey.toByteVector.toBase64,
                nodeAddress,
                masterConf.endpoints.ip.getHostAddress,
                10,
                10
              )
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← eventually[IO](checkAlive(node.pool, 1), 100.millis, 25.seconds)

            id0 ← node.pool.listAll().map(_.head.app.id)

            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← eventually[IO](checkAlive(node.pool, 2), 100.millis, 25.seconds)

            _ ← contract.deleteApp[IO](id0)
            _ ← eventually[IO](checkDestroyed(node.pool, id0), 100.millis, 25.seconds)

            id1 ← node.pool.listAll().map(_.filter(_.app.id != id0).head.app.id)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← contract.deleteApp[IO](id1)
            _ ← eventually[IO](checkDestroyed(node.pool, id1), 100.millis, 25.seconds)

            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)

            _ ← eventually[IO](checkAlive(node.pool, 8), 100.millis, 25.seconds)

            id2 ← node.pool.listAll().map(_.last.app.id)
            _ ← contract.deleteApp[IO](id2)

            _ ← eventually[IO](checkAlive(node.pool, 7), 100.millis, 15.seconds)

          } yield ()
      }.unsafeRunSync()
    }

    "form a kademlia network" in {
      implicit val log: Log[IO] = logFactory.init("spec", "kad", Log.Trace).unsafeRunSync()

      runningNode().flatMap {
        case (s, mn, _, kademlia) ⇒
          val seed = kademlia.ownContact.map(_.contact.toString).unsafeRunSync()

          implicit val ss = s
          eventually[IO](
            sttp.post(uri"http://127.0.0.1:5789/kad/ping").send().value.map(_.flatMap(_.body).right.get shouldBe seed)
          )

          runningNode(5679, seed :: Nil).map(_._4 -> kademlia)
      }.use {
        case (kad1, kad0) ⇒
          eventually(
            Apply[IO].map2(
              kad1.findNode(kad0.nodeKey, 2),
              kad0.findNode(kad1.nodeKey, 2)
            ) {
              case (f1, f0) ⇒
                f1 shouldBe defined
                f0 shouldBe defined
            },
            maxWait = 10.seconds
          )
      }.unsafeRunSync()
    }
  }
}
