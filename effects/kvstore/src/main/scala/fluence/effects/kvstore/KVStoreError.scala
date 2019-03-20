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

package fluence.effects.kvstore
import fluence.codec.CodecError
import fluence.effects.EffectError

sealed trait KVStoreError extends EffectError

sealed trait KVReadError extends KVStoreError

sealed trait KVWriteError extends KVStoreError

case class ValueCodecError(error: CodecError) extends KVReadError with KVWriteError {
  override def getCause: Throwable = error
}

case class KeyCodecError(error: CodecError) extends KVReadError with KVWriteError {
  override def getCause: Throwable = error
}

case class IOExceptionError(message: String, cause: Throwable) extends KVReadError with KVWriteError {
  override def getCause: Throwable = Option(cause) getOrElse super.getCause
}
