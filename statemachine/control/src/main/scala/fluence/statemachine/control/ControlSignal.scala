package fluence.statemachine.control
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import scodec.bits.ByteVector

// Common trait for all control signals from node to worker
sealed trait ControlSignal

/**
 * A signal to change a voting power of the specified Tendermint validator. Voting power zero votes to remove.
 * Represents a Tendermint's ValidatorUpdate command
 * see https://github.com/tendermint/tendermint/blob/master/docs/spec/abci/abci.md#validatorupdate
 *
 * @param keyType Type of a Tendermint node's validator key. Currently always PubKeyEd25519.
 * @param validatorKey Validator key's bytes
 * @param votePower Desired resulting vote power of the validator. Zero votes to remove that node from validators set.
 */
case class ChangePeer(keyType: String, validatorKey: ByteVector, votePower: Long) extends ControlSignal

object ChangePeer {
  implicit val dec: Decoder[ChangePeer] = deriveDecoder[ChangePeer]
  private implicit val decbc: Decoder[ByteVector] =
    Decoder.decodeString.flatMap(
      ByteVector.fromHex(_).fold(Decoder.failedWithMessage[ByteVector]("Not a hex"))(Decoder.const)
    )

  implicit val enc: Encoder[ChangePeer] = deriveEncoder[ChangePeer]
  private implicit val encbc: Encoder[ByteVector] = Encoder.encodeString.contramap(_.toHex)
}
