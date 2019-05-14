package fluence.effects.node.rpc

import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import scodec.bits.ByteVector

case class UploadBlock(height: Long, vmHash: ByteVector, previousManifestReceipt: Option[Receipt])

object UploadBlock {
  implicit val dec: Decoder[UploadBlock] = deriveDecoder[UploadBlock]
  implicit val enc: Encoder[UploadBlock] = deriveEncoder[UploadBlock]
}