package fluence.statemachine.data

object TxCode extends Enumeration {
  val OK, BAD, BadNonce, QueueDropped, AlreadyProcessed = Value
}
