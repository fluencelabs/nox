package fluence.client

sealed trait Operation
case object NoOp extends Operation
case class Put(key: String, value: String) extends Operation
case class Get(key: String) extends Operation
case object Exit extends Operation
