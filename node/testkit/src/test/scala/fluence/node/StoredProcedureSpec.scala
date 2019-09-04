package fluence.node

import cats.effect.{ContextShift, IO, Timer}
import fluence.crypto.{Crypto, CryptoError}
import fluence.effects.tendermint.rpc.http.TendermintHttpRpc
import fluence.effects.tendermint.rpc.websocket.{TestTendermintWebsocketRpc, WebsocketConfig}
import fluence.log.{Log, LogFactory}
import fluence.node.workers.subscription.{
  StoredProcedureExecutor,
  TendermintQueryResponse,
  TxAwaitError,
  WaitResponseService
}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.util.Try

class StoredProcedureSpec {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)
  implicit private val logFactory = LogFactory.forPrintln[IO](level = Log.Error)
  implicit private val log = logFactory.init("ResponseSubscriberSpec", level = Log.Off).unsafeRunSync()

  val tendermintWrpc = new TestTendermintWebsocketRpc[IO] {}

  val waitResponseService = new WaitResponseService[IO] {
    override def sendTxAwaitResponse(tx: String, id: Option[String])(
      implicit log: Log[IO]
    ): IO[Either[TxAwaitError, TendermintQueryResponse]] = ???
  }

  val hasher: Crypto.Hasher[Array[Byte], String] = Crypto.liftFuncEither(
    bytes ⇒
      Try {
        new String(bytes)
      }.toEither.left
        .map(err ⇒ CryptoError(s"Unexpected", Some(err)))
  )

  val storedProcedureExecutor = StoredProcedureExecutor[IO](tendermintWrpc, tendermintWrpc, waitResponseService, hasher)
}
