package fluence.dataset.client

trait Flow[T]
case class Continuation[T](reply: T) extends Flow[T]
case class Result[T](result: Option[Array[Byte]]) extends Flow[T]
case class RangeResult[T](key: Array[Byte], value: Array[Byte]) extends Flow[T]
case class ErrorFromClient[T](reply: T) extends Flow[T]
case object Stop extends Flow[Any]
