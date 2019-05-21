package fluence.node.workers.control

import fluence.effects.tendermint.block.history.Receipt
import scodec.bits.ByteVector

import scala.util.control.NoStackTrace

trait ControlRpcError extends NoStackTrace

trait WithCause[E <: Throwable] extends ControlRpcError {
  def cause: E

  initCause(cause)
}

case class DropPeerError(key: ByteVector, cause: Throwable) extends WithCause[Throwable] {
  override def getMessage: String = s"Error dropping peer ${key.toHex}"
}
case class StatusError(cause: Throwable) extends WithCause[Throwable] {
  override def getMessage: String = "Error retrieving worker status"
}
case class StopError(cause: Throwable) extends WithCause[Throwable] {
  override def getMessage: String = "Error while signaling worker to stop"
}
case class SendBlockReceiptError(receipt: Receipt, cause: Throwable) extends WithCause[Throwable] {
  override def getMessage: String = s"Error sending block receipt ${receipt.hash.toHex}"
}
case class GetVmHashError(cause: Throwable) extends WithCause[Throwable] {
  override def getMessage: String = "Error getting VM hash from worker"
}
