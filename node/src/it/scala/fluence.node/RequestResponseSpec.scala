package fluence.node

import java.net.InetAddress
import java.nio.file.Paths

import cats.Parallel
import cats.data.NonEmptyList
import cats.effect.concurrent.Ref
import cats.effect.{ContextShift, IO, Resource, Timer}
import fluence.effects.docker.params.{DockerImage, DockerLimits}
import fluence.log.{Log, LogFactory}
import fluence.node.config.DockerConfig
import fluence.node.eth.state.{App, Cluster, StorageRef, StorageType, WorkerPeer}
import fluence.node.workers.{TxSyncErrorT, WorkerParams, WorkersApi, WorkersPool}
import fluence.node.workers.subscription.{
  RequestResponderImpl,
  RequestSubscriber,
  ResponsePromise,
  TendermintQueryResponse
}
import fluence.node.workers.tendermint.config.{ConfigTemplate, TendermintConfig}
import org.scalatest.{BeforeAndAfterAll, Matchers, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.duration._
import scala.compat.Platform.currentTime
import scala.concurrent.ExecutionContext.Implicits.global

class RequestResponseSpec extends WordSpec with Matchers with BeforeAndAfterAll {

  def start() = {
    implicit val ioTimer: Timer[IO] = IO.timer(global)
    implicit val ioShift: ContextShift[IO] = IO.contextShift(global)
    implicit val logFactory = LogFactory.forPrintln[IO]()
    implicit val log = logFactory.init("RequestResponseSpec", level = Log.Trace).unsafeRunSync()

    val ref = Ref.unsafe[IO, Map[Long, NonEmptyList[ResponsePromise[IO]]]](
      Map.empty[Long, NonEmptyList[ResponsePromise[IO]]]
    )
    val requestSubscriber = RequestSubscriber(ref)
    val requestResponder = RequestResponderImpl[IO, IO.Par](ref, 3)

    val rootPath = Paths.get("/tmp")

    val appId = 1L
    val p2pPort = 10001.toShort
    val workerPeer = WorkerPeer(ByteVector.empty, "", 25000.toShort, InetAddress.getLocalHost, 0)
    val cluster = Cluster(currentTime.millis, Vector.empty, workerPeer)
    val app = App(appId, StorageRef(ByteVector.empty, StorageType.Ipfs), cluster)
    val dockerConfig = DockerConfig(DockerImage("fluencelabs/worker", "v0.2.0"), DockerLimits(None, None, None))
    val tmDockerConfig = DockerConfig(DockerImage("tendermint/tendermint", "v0.32.0"), DockerLimits(None, None, None))
    val tmConfig = TendermintConfig("info", 0, 0, 0, 0L, false, false, false, p2pPort, Seq.empty)
    val configTemplate = ConfigTemplate[IO](rootPath, tmConfig).unsafeRunSync()
    val params = WorkerParams(app, rootPath, rootPath, None, dockerConfig, tmDockerConfig, configTemplate)

    for {
      pool <- Resource.liftF(TestWorkersPool.some[IO])
      _ <- Resource.liftF(pool.run(appId, IO(params)))
      _ <- requestResponder.subscribeForWaitingRequests(pool.get(1).unsafeRunSync().get)
    } yield (pool, requestSubscriber, log, ioShift)
  }

  def tx(nonce: Int) = {
    s"""|asdf/$nonce
        |this_should_be_a_llamadb_signature_but_it_doesnt_matter_for_this_test
        |1
        |INSERT INTO users VALUES(1, 'Sara', 23), (2, 'Bob', 19), (3, 'Caroline', 31), (4, 'Max', 27)
        |""".stripMargin
  }

  def requests(to: Int, pool: WorkersPool[IO], requestSubscriber: RequestSubscriber[IO])(
    implicit P: Parallel[IO, IO.Par],
    log: Log[IO]
  ): IO[List[Either[TxSyncErrorT, TendermintQueryResponse]]] = {
    import cats.instances.list._
    import cats.syntax.parallel._

    Range(0, to).toList.map { nonce =>
      WorkersApi.txWaitResponse[IO, IO.Par](pool, requestSubscriber, 1, tx(nonce), None)
    }.parSequence
  }

  "MasterNodes" should {
    "sync their workers with contract clusters" in {

      start().use {
        case (pool, requestSubscriber, log, ioShift) =>
          implicit val io = ioShift
          implicit val l = log
          requests(20, pool, requestSubscriber).map(r => println(r.mkString("\n")))
      }.unsafeRunSync()
    }
  }
}
