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

package fluence.bp.uploading

import fluence.effects.ipfs.IpfsData
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.statemachine.api.data.BlockReceipt
import org.scalatest.{Matchers, OptionValues}
import scodec.bits.ByteVector

case class UploadingState(
  uploads: Int = 0,
  vmHashGet: Seq[Long] = Nil,
  receipts: Seq[BlockReceipt] = Vector.empty,
  lastKnownHeight: Option[Long] = None,
  blockManifests: Seq[BlockManifest] = Nil
) {

  def upload[A: IpfsData](data: A) = data match {
    case d: ByteVector =>
      val manifests = BlockManifest.fromBytes(d).fold(_ => blockManifests, blockManifests :+ _)
      copy(uploads = uploads + 1, blockManifests = manifests)
    case _ =>
      copy(uploads = uploads + 1)
  }

  def vmHash(height: Long) = copy(vmHashGet = vmHashGet :+ height)

  def receipt(receipt: BlockReceipt) =
    copy(receipts = receipts :+ receipt)

  def subscribe(lastKnownHeight: Long) = copy(lastKnownHeight = Some(lastKnownHeight))
}
