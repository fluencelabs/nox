package fluence.effects.ipfs
import fluence.effects.castore.StoreError

case class IpfsError(message: String, causedBy: Option[Throwable] = None) extends StoreError {
  override def getMessage: String = message

  override def getCause: Throwable = causedBy getOrElse super.getCause
}