package fluence.statemachine.client

import cats.Monad
import cats.data.EitherT
import cats.syntax.functor._
import com.softwaremill.sttp.{sttp, _}
import fluence.effects.EffectError
import fluence.effects.sttp.SttpEffect
import fluence.effects.sttp.syntax._
import fluence.log.Log
import fluence.statemachine.api.command.PeersControl
import scodec.bits.ByteVector

import scala.language.higherKinds

class PeersControlClient[F[_] : Monad : SttpEffect](host: String, port: Short) extends PeersControl[F] {

  override def dropPeer(validatorKey: ByteVector)(implicit log: Log[F]): EitherT[F, EffectError, Unit] =
    sttp
      .post(uri"http://$host:$port/peers/drop")
      .body(validatorKey.toHex)
      .send()
      .toBody
      .void
      .leftMap(identity[EffectError])
}
