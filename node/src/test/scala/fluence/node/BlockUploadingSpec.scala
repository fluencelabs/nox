package fluence.node

import java.nio.file.Paths

import cats.data.EitherT
import cats.effect.{ContextShift, IO, Timer}
import fluence.node.config.storage.{IpfsConfig, RemoteStorageConfig, SwarmConfig}
import fluence.node.workers.tendermint.BlockUploading
import org.scalatest.WordSpec
import com.softwaremill.sttp.{SttpBackend, _}
import fluence.EitherTSttpBackend
import fluence.effects.castore.StoreError
import fluence.effects.ipfs.{IpfsData, IpfsUploader}
import fluence.log.{Log, LogFactory}
import fluence.node.config.DockerConfig
import fluence.node.eth.state.{App, StorageRef, StorageType}
import fluence.node.workers.{DockerWorkerServices, Worker, WorkerParams, WorkerServices}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global

class BlockUploadingSpec extends WordSpec {
  implicit private val timer = IO.timer(global)
  implicit private val shift = IO.contextShift(global)
  implicit private val log = LogFactory.forPrintln[IO]().init("block uploading spec").unsafeRunSync()
  implicit private val sttp = EitherTSttpBackend[IO]()

  private val rmc =
    RemoteStorageConfig(true, SwarmConfig(uri"http://swarmhost:11234"), IpfsConfig(uri"http://ipfshost:44321"))
  private val rootPath = Paths.get("/tmp")
  private val ipfs = new IpfsUploader[IO] {
    override def upload[A: IpfsData](data: A)(implicit log: Log[IO]): EitherT[IO, StoreError, ByteVector] =
      EitherT.pure(ByteVector.empty)
  }

  private val blockUploading = BlockUploading.make[IO](ipfs, rootPath)

  val appId = 1L
  val p2pPort = 10001.toShort
  val description = "worker #1"
  val app = App(123L, StorageRef(ByteVector.empty, StorageType.Ipfs))
  val params = WorkerParams(app, rootPath, rootPath, None, DockerConfig)
  val workerServices: WorkerServices[IO] = DockerWorkerServices.make()
  val worker: Worker[IO] = Worker.make(appId, p2pPort, description, )

  blockUploading.start()
}
