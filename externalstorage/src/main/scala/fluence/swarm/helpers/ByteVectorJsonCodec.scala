package fluence.swarm.helpers
import io.circe.{Decoder, Encoder, Json}
import scodec.bits.ByteVector

object ByteVectorJsonCodec {

  /**
   * Every byte array in JSON is `0x` prefixed in Swarm.
   */
  implicit final val encodeByteVector: Encoder[ByteVector] = (bv: ByteVector) => Json.fromString("0x" + bv.toHex)

  implicit final val decodeByteVector: Decoder[ByteVector] = {
    Decoder.decodeString.emap { str =>
      ByteVector.fromHexDescriptive(str).left.map(t => "ByteVector")
    }
  }
}
