package fluence.statemachine.control
import io.circe.generic.JsonCodec
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import scodec.bits.ByteVector

/**
 * Common trait for all control signals from node to worker
 */
sealed trait ControlSignal

/**
 * Asks worker for it's status
 */
case class GetStatus() extends ControlSignal

object GetStatus {
  implicit val dec: Decoder[GetStatus] = deriveDecoder[GetStatus]
  implicit val enc: Encoder[GetStatus] = deriveEncoder[GetStatus]
}

/**
 * Tells worker to stop
 */
case class Stop() extends ControlSignal

object Stop {
  implicit val dec: Decoder[Stop] = deriveDecoder[Stop]
  implicit val enc: Encoder[Stop] = deriveEncoder[Stop]
}

/**
 * A signal to drop Tendermint validator by setting it's voting power to zero
 * Used to build a Tendermint's ValidatorUpdate command
 * see https://github.com/tendermint/tendermint/blob/master/docs/spec/abci/abci.md#validatorupdate
 *
 * @param validatorKey Validator key's bytes
 */
case class DropPeer(validatorKey: ByteVector) extends ControlSignal

object DropPeer {
  // Type of a Tendermint node's validator key. Currently always PubKeyEd25519.
  val KEY_TYPE = "PubKeyEd25519"

  implicit val dec: Decoder[DropPeer] = deriveDecoder[DropPeer]
  private implicit val decbc: Decoder[ByteVector] =
    Decoder.decodeString.flatMap(
      ByteVector.fromHex(_).fold(Decoder.failedWithMessage[ByteVector]("Not a hex"))(Decoder.const)
    )

  implicit val enc: Encoder[DropPeer] = deriveEncoder[DropPeer]
  private implicit val encbc: Encoder[ByteVector] = Encoder.encodeString.contramap(_.toHex)
}
