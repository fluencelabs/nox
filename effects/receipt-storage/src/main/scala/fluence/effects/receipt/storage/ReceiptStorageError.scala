package fluence.effects.receipt.storage

import fluence.effects.{EffectError, WithCause}
import fluence.effects.kvstore.{KVReadError, KVWriteError}

trait ReceiptStorageError extends EffectError

case class PutError(id: Long, height: Long, cause: KVWriteError) extends WithCause[KVWriteError] {
  override def getMessage: String = s"receipt put error app $id height $height: $cause"
}

case class GetError(id: Long, height: Long, cause: KVReadError) extends WithCause[KVReadError] {
  override def getMessage: String = s"receipt get error app $id height $height: $cause"
}

case class RetrieveError(id: Long, from: Option[Long], to: Option[Long])