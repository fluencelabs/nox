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

import cats.data.EitherT
import cats.instances.option._
import cats.{Monad, Traverse}
import fluence.effects.ipfs.IpfsClient
import fluence.effects.ipfs.IpfsData._
import fluence.effects.tendermint.block.data.Block
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Implements block manifest uploading as described in Fluence paper
  *
  * 1. Upload transactions, get receipt
  * 2. Upload BlockManifest, using receipt from step 1, get receipt
 *
 * @param ipfs Decentralized storage, currently IPFS
 */
case class BlockHistory[F[_]: Monad](ipfs: IpfsClient[F]) {

  def upload(
    block: Block,
    vmHash: ByteVector,
    previousManifestReceipt: Option[Receipt]
  ): EitherT[F, BlockHistoryError, Receipt] = {
    val txs = block.data.txs.map(_.map(_.bv))
    val votes = block.last_commit.precommits.flatten
    val height = block.header.height
    for {
      // TODO: what to return on txs=None?
      txsReceipt <- Traverse[Option].sequence(txs.map(uploadTxs(height, _)))
      manifest = BlockManifest(vmHash, previousManifestReceipt, txsReceipt, block.header, votes)
      receipt <- uploadManifest(height, manifest)
    } yield receipt
  }

  private def uploadTxs(height: Long, txs: List[ByteVector]): EitherT[F, BlockHistoryError, Receipt] =
    ipfs
      .upload(txs)
      .map(Receipt(_))
      .leftMap(se => TxsUploadingError(height, txs.size, se): BlockHistoryError)

  private def uploadManifest(height: Long, manifest: BlockManifest): EitherT[F, BlockHistoryError, Receipt] =
    ipfs
      .upload(manifest.bytes())
      .map(Receipt(_))
      .leftMap(se => ManifestUploadingError(height, se): BlockHistoryError)
}
