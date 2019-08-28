package fluence.effects.tendermint.block.history.db

import fluence.effects.{EffectError, WithCause}

trait BlockstoreError extends EffectError
case class DbNotFound(path: String) extends BlockstoreError {
  override def toString: String = s"DbNotFound: can't find leveldb database at path $path: path doesn't exist"
}
case object NoTmpDirPropertyError extends BlockstoreError {
  override def toString: String = "NoTmpDirPropertyError: java.io.tmpdir is empty"
}
case class GetBlockError(message: String, height: Long) extends BlockstoreError {
  override def toString: String = s"GetBlockError: $message on block $height"
}
case class SymlinkCreationError(cause: Throwable, target: String) extends BlockstoreError with WithCause[Throwable] {
  override def toString: String = s"SymlinkCreationError: error creating symlink for $target: $cause"
}
case class RetrievingStorageHeightError(cause: String) extends BlockstoreError {
  override def toString: String = s"RetrievingStorageHeightError: $cause"
}

object RetrievingStorageHeightError {
  def apply(cause: Throwable): RetrievingStorageHeightError = new RetrievingStorageHeightError(cause.toString)
}
