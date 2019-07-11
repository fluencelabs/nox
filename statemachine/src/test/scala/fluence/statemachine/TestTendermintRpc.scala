package fluence.statemachine

import cats.Functor
import cats.data.EitherT
import cats.effect.IO
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.http.{RpcError, TendermintHttpRpc}
import fluence.effects.tendermint.rpc.response.TendermintStatus

class TestTendermintRpc extends TendermintHttpRpc[IO] {
  override def status: EitherT[IO, RpcError, String] = throw new NotImplementedError("def status")

  override def statusParsed(implicit F: Functor[IO]): EitherT[IO, RpcError, TendermintStatus] =
    throw new NotImplementedError("def statusParsed")

  override def block(height: Long, id: String): EitherT[IO, RpcError, Block] =
    throw new NotImplementedError("def block")

  override def commit(height: Long, id: String): EitherT[IO, RpcError, String] =
    throw new NotImplementedError("def commit")

  override def consensusHeight(id: String): EitherT[IO, RpcError, Long] =
    throw new NotImplementedError("def consensusHeight")

  override def broadcastTxSync(tx: String, id: String): EitherT[IO, RpcError, String] =
    throw new NotImplementedError("def broadcastTxSync")

  override def unsafeDialPeers(peers: Seq[String], persistent: Boolean, id: String): EitherT[IO, RpcError, String] =
    throw new NotImplementedError("def unsafeDialPeers")

  override def query(path: String,
                     data: String,
                     height: Long,
                     prove: Boolean,
                     id: String): EitherT[IO, RpcError, String] = throw new NotImplementedError("def query")
}
