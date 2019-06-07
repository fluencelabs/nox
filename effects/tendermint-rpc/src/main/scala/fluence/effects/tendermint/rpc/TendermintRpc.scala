package fluence.effects.tendermint.rpc

import cats.Monad
import cats.data.EitherT
import cats.effect.{ConcurrentEffect, Timer}
import com.softwaremill.sttp.SttpBackend
import fluence.effects.tendermint.rpc.http.TendermintHttpRpcImpl
import fluence.effects.tendermint.rpc.websocket.WebsocketTendermintRpcImpl

import scala.language.higherKinds

class TendermintRpc[F[_]: ConcurrentEffect: Timer: Monad](host: String, port: Int)(
  implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing]
) extends TendermintHttpRpcImpl[F](host, port) with WebsocketTendermintRpcImpl[F]
