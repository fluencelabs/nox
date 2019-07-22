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

package fluence.statemachine.control

trait HasHeight[A] {
  def height(a: A): Long
}

object HasHeight {
  implicit val vmHash: HasHeight[VmHash] = (vh: VmHash) => vh.height
  implicit val receipt: HasHeight[BlockReceipt] = (br: BlockReceipt) => br.receipt.height

  def apply[A: HasHeight]: HasHeight[A] = implicitly[HasHeight[A]]

  object syntax {
    implicit class HasHeightSyntax[A: HasHeight](a: A) {
      def height: Long = HasHeight[A].height(a)
    }
  }
}
