package fluence.statemachine.client

import cats.Monad
import cats.data.EitherT
import cats.syntax.functor._
import fluence.effects.EffectError
import fluence.effects.sttp.SttpEffect
import fluence.effects.sttp.syntax._
import com.softwaremill.sttp.{sttp, _}
import fluence.log.Log
import fluence.statemachine.api.command.HashesBus
import fluence.statemachine.api.data.BlockReceipt
import scodec.bits.ByteVector
import com.softwaremill.sttp.circe._

import scala.language.higherKinds

class HashesBusClient[F[_]: Monad: SttpEffect](host: String, port: Short) extends HashesBus[F]{

  override def getVmHash(height: Long)(implicit log: Log[F]): EitherT[F, EffectError, ByteVector] =
    sttp
      .get(uri"http://$host:$port/hashes-bus/getVmHash?height=$height")
    .send()
    .decodeBody(ByteVector.fromHex(_).toRight(new RuntimeException("Not a hex")))
    .leftMap(identity[EffectError])

  override def sendBlockReceipt(receipt: BlockReceipt)(implicit log: Log[F]): EitherT[F, EffectError, Unit] =
    sttp
    .post(uri"http://$host:$port/hashes-bus/blockReceipt")
    .body(receipt)
    .send()
    .toBody
    .void
      .leftMap(identity[EffectError])
}
