package fluence.node.eth.state
import io.circe.{Decoder, Encoder}

/**
 * Type of the decentralized storage
 */
object StorageType extends Enumeration {
  type StorageType = Value

  val Swarm, Ipfs = Value

  def toByte(storageType: StorageType): Byte = storageType match {
    case StorageType.Swarm => 0
    case StorageType.Ipfs => 1
  }

  implicit val decoder: Decoder[StorageType] = Decoder.enumDecoder(StorageType)
  implicit val encoder: Encoder[StorageType] = Encoder.enumEncoder(StorageType)
}
