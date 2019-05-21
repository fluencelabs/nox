package fluence.effects.tendermint.block.history.helpers

import io.circe.{Decoder, Encoder}
import scodec.bits.ByteVector

object ByteVectorJsonCodec {
  implicit val decbc: Decoder[ByteVector] =
    Decoder.decodeString.flatMap(
      ByteVector.fromHex(_).fold(Decoder.failedWithMessage[ByteVector]("Not a hex"))(Decoder.const)
    )

  implicit val encbc: Encoder[ByteVector] = Encoder.encodeString.contramap(_.toHex)
}
