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

package fluence.bp.tx

import scodec.bits.ByteVector

trait TxsBlock[B] {
  def height(block: B): Long
  def txs(block: B): Seq[ByteVector]
}

object TxsBlock {
  def apply[B](implicit tb: TxsBlock[B]): TxsBlock[B] = tb
}
