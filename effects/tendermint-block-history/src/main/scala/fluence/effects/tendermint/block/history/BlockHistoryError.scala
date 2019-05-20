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

package fluence.effects.tendermint.block.history

import fluence.effects.EffectError
import fluence.effects.castore.StoreError

import scala.util.control.NoStackTrace

trait BlockHistoryError extends EffectError with NoStackTrace

trait WithCause[E <: Throwable] extends BlockHistoryError {
  def cause: E

  initCause(cause)
}

case class ManifestUploadingError(height: Long, cause: StoreError) extends WithCause[StoreError] {
  override def getMessage: String = s"Error uploading manifest at height $height: ${cause.getMessage}"
}

case class TxsUploadingError(height: Long, txsCount: Int, cause: StoreError) extends WithCause[StoreError] {
  override def getMessage: String = s"Error uploading $txsCount txs at height $height: ${cause.getMessage}"
}
