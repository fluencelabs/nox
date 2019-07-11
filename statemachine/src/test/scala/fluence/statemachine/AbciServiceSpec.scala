package fluence.statemachine

import cats.data.EitherT
import cats.effect.concurrent.Ref
import cats.effect.{IO, Resource}
import cats.syntax.compose._
import fluence.crypto.Crypto
import fluence.crypto.hash.JdkCryptoHasher
import fluence.log.{Log, LogFactory}
import fluence.statemachine.control.{BlockReceipt, ControlSignals, DropPeer}
import fluence.statemachine.error.StateMachineError
import fluence.statemachine.state.AbciState
import fluence.statemachine.vm.VmOperationInvoker
import org.scalatest.WordSpec
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global
import scala.language.higherKinds

class AbciServiceSpec extends WordSpec {
  implicit private val timer = IO.timer(global)
  implicit private val shift = IO.contextShift(global)
  implicit private val log = LogFactory.forPrintln[IO]().init("block uploading spec", level = Log.Warn).unsafeRunSync()

  val vmInvoker = new VmOperationInvoker[IO] {
    override def invoke(arg: Array[Byte]): EitherT[IO, StateMachineError, Array[Byte]] = ???
    override def vmStateHash(): EitherT[IO, StateMachineError, ByteVector] = ???
  }

  val controlSignals = new ControlSignals[IO] {
    override val dropPeers: Resource[IO, Set[DropPeer]] = ???
    override val stop: IO[Unit] = ???
    override val receipt: IO[BlockReceipt] = ???
    override def putVmHash(hash: ByteVector): IO[Unit] = ???
    override def setVmHash(hash: ByteVector): IO[Unit] = ???
  }

  val tendermintRpc = new TestTendermintRpc

  implicit val hasher: Crypto.Hasher[ByteVector, ByteVector] = {
    val bva = Crypto.liftFunc[ByteVector, Array[Byte]](_.toArray)
    val abv = Crypto.liftFunc[Array[Byte], ByteVector](ByteVector(_))
    bva.andThen[Array[Byte]](JdkCryptoHasher.Sha256).andThen(abv)
  }

  val abciService = for {
    state ← Ref.of[IO, AbciState](AbciState())
    abci = new AbciService[IO](state, vmInvoker, controlSignals, tendermintRpc)
  } yield abci

  "" should {
    "" in {}
  }
}
