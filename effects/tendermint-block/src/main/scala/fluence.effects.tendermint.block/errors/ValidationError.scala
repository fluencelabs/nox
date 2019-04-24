package fluence.effects.tendermint.block.errors

object ValidationError {
  sealed trait ValidationError extends TendermintBlockError
  case class InvalidDataHash(expected: String, actual: String) extends ValidationError
  case class InvalidCommitHash(expected: String, actual: String) extends ValidationError
}
