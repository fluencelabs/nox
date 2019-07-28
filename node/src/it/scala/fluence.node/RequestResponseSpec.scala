package fluence.node

import java.net.InetAddress
import java.nio.file.Paths

import cats.{effect, Parallel}
import cats.data.NonEmptyList
import cats.effect.concurrent.Ref
import cats.effect.{ContextShift, IO, Resource, Timer}
import fluence.effects.docker.params.{DockerImage, DockerLimits}
import fluence.log.{Log, LogFactory}
import cats.syntax.applicative._
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.node.config.DockerConfig
import fluence.node.eth.state.{App, Cluster, StorageRef, StorageType, WorkerPeer}
import fluence.node.workers.{TxAwaitError, WorkerParams, WorkersApi, WorkersPool}
import fluence.node.workers.subscription.{
  RequestResponder,
  RequestResponderImpl,
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
      tendermint <- Resource.liftF(TendermintTest[IO]())
      requestResponder <- Resource
        .liftF[IO, RequestResponderImpl[IO, effect.IO.Par]](
          RequestResponderImpl[IO, IO.Par](tendermint.tendermint, appId)
        )
      pool <- Resource.liftF(TestWorkersPool.some[IO](requestResponder, tendermint.tendermint))
      _ <- Resource.liftF(pool.run(appId, IO(params)))
      _ <- requestResponder.subscribeForWaitingRequests()
    } yield (pool, requestResponder, log, ioShift, ioTimer, tendermint)
  }

  def tx(nonce: Int) = {
    s"""|asdf/$nonce
        |this_should_be_a_llamadb_signature_but_it_doesnt_matter_for_this_test
        |1
        |INSERT INTO users VALUES(1, 'Sara', 23), (2, 'Bob', 19), (3, 'Caroline', 31), (4, 'Max', 27)
        |""".stripMargin
  }

  def requests(to: Int, pool: WorkersPool[IO], requestSubscriber: RequestResponder[IO])(
    implicit P: Parallel[IO, IO.Par],
    log: Log[IO]
  ): IO[List[Either[TxAwaitError, TendermintQueryResponse]]] = {
    import cats.instances.list._
    import cats.syntax.parallel._

    Range(0, to).toList.map { nonce =>
      WorkersApi.txWaitResponse[IO, IO.Par](pool, 1, tx(nonce), None)
    }.parSequence
  }

  "MasterNodes" should {
    "sync their workers with contract clusters" in {

      start().use {
        case (pool, requestSubscriber, log, ioShift, ioTimer, tendermintTest) =>
          implicit val io = ioShift
          implicit val timer = ioTimer
          implicit val l = log
          for {
            _ <- requests(40, pool, requestSubscriber).map(r => println(r.count(_.isRight)))
            _ <- IO.sleep(700.millis)
            _ <- requests(30, pool, requestSubscriber).map(r => println(r.count(_.isRight)))
            _ <- IO.sleep(500.millis)
            _ <- requests(70, pool, requestSubscriber).map(r => println(r.count(_.isRight)))
          } yield ()
      }.unsafeRunSync()
    }
  }
}
