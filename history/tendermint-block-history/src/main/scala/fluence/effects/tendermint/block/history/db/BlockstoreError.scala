/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.effects.tendermint.block.history.db

import java.nio.file.Path

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
case class SymlinkCreationError(cause: Throwable, target: Path) extends BlockstoreError with WithCause[Throwable] {
  override def toString: String = s"SymlinkCreationError: error creating symlink for $target: $cause"
}
case class RetrievingStorageHeightError(cause: String) extends BlockstoreError {
  override def toString: String = s"RetrievingStorageHeightError: $cause"
}
case class OpenDbError(cause: Throwable) extends BlockstoreError with WithCause[Throwable] {
  override def toString: String = s"OpenDbError: error opening db $cause"
}

object RetrievingStorageHeightError {
  def apply(cause: Throwable): RetrievingStorageHeightError = new RetrievingStorageHeightError(cause.toString)
}
