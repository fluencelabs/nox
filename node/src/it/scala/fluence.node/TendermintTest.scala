package fluence.node

import cats.{Functor, Monad}
import cats.data.EitherT
import cats.effect.Timer
import fluence.effects.{Backoff, EffectError}
import fluence.effects.tendermint.block.TestData
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.effects.tendermint.rpc.http.RpcError
import fluence.effects.tendermint.rpc.websocket.{TestTendermintRpc, TestTendermintWebsocketRpc}
import fluence.log.Log

import scala.concurrent.duration._
import scala.language.higherKinds

object TendermintTest {

  def requestResponderTendermint[F[_]: Timer: Monad](): TendermintRpc[F] =
    new TestTendermintRpc[F] with TestTendermintWebsocketRpc[F] {
      override def subscribeNewBlock(lastKnownHeight: Long)(implicit log: Log[F],
                                                            backoff: Backoff[EffectError]): fs2.Stream[F, Block] =
        fs2.Stream
          .awakeEvery[F](500.milliseconds)
          .map(_ => Block(TestData.blockWithNullTxsResponse(1)).right.get)

      override def consensusHeight(id: String): EitherT[F, RpcError, Long] =
        EitherT.pure(0)

      override def broadcastTxSync(tx: String, id: String): EitherT[F, RpcError, String] =
        EitherT.pure("""
                       |{
                       |
                       |
                       |    "error": "",
                       |    "result": {
                       |        "hash": "2B8EC32BA2579B3B8606E42C06DE2F7AFA2556EF",
                       |        "log": "",
                       |        "data": "",
                       |        "code": "0"
                       |    },
                       |    "id": "",
                       |    "jsonrpc": "2.0"
                       |
                       |}
                       |""".stripMargin)

      override def query(
        path: String,
        data: String,
        height: Long,
        prove: Boolean,
        id: String
      ): EitherT[F, RpcError, String] = EitherT.pure("""
                                                       |{
                                                       |
                                                       |
                                                       |    "error": "",
                                                       |    "result": {
                                                       |        "response": {
                                                       |            "log": "exists",
                                                       |            "height": "0",
                                                       |            "value": "61626364",
                                                       |            "key": "61626364",
                                                       |            "index": "-1",
                                                       |            "code": "0"
                                                       |        }
                                                       |    },
                                                       |    "id": "",
                                                       |    "jsonrpc": "2.0"
                                                       |
                                                       |}
                                                       |""".stripMargin)
    }
}
