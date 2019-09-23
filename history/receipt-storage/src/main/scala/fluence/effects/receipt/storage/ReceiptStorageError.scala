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

package fluence.effects.receipt.storage

import fluence.effects.{EffectError, WithCause}
import fluence.effects.kvstore.{KVReadError, KVWriteError}

trait ReceiptStorageError extends EffectError

case class PutError(id: Long, height: Long, cause: KVWriteError)
    extends ReceiptStorageError with WithCause[KVWriteError] {
  override def getMessage: String = s"receipt put error app $id height $height: $cause"
}

case class GetError(id: Long, height: Long, cause: KVReadError)
    extends ReceiptStorageError with WithCause[KVReadError] {
  override def getMessage: String = s"receipt get error app $id height $height: $cause"
}
