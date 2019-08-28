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
import cats.{Applicative, Monad, Traverse}
import fluence.effects.ipfs.IpfsUploader
import fluence.effects.ipfs.IpfsData._
import fluence.effects.tendermint.block.data.Block
import fluence.log.Log
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
class BlockHistory[F[_]: Monad](ipfs: IpfsUploader[F]) {

  /**
   * Uploads block manifest.
   *
   * @param block The block to upload
   * @param vmHash Hash of the VM state after this block
   * @param previousManifestReceipt Receipt of the manifest for previous block
   * @param emptyBlocksReceipts Receipts for empty blocks preceding the current `block`
   * @param onManifestUploaded Hook to trigger once a new manifest is uploaded
   * @return Block manifest receipt
   */
  def upload(
    block: Block,
    vmHash: ByteVector,
    previousManifestReceipt: Option[Receipt],
    emptyBlocksReceipts: List[Receipt],
    onManifestUploaded: (BlockManifest, Receipt) ⇒ F[Unit] = (_, _) ⇒ Applicative[F].unit
  )(implicit log: Log[F]): EitherT[F, BlockHistoryError, Receipt] = {
    val txs = block.data.txs.filter(_.nonEmpty).map(_.map(_.bv))
    val votes = block.last_commit.precommits.flatten
    val height = block.header.height
    for {
      txsReceipt <- Traverse[Option].sequence(txs.map(uploadTxs(height, _)))
      manifest = BlockManifest(vmHash, previousManifestReceipt, txsReceipt, block.header, votes, emptyBlocksReceipts)
      receipt <- uploadManifest(height, manifest)
      _ ← EitherT.right(onManifestUploaded(manifest, receipt))
    } yield receipt
  }

  // TODO write docs
  private def uploadTxs(height: Long, txs: List[ByteVector])(
    implicit log: Log[F]
  ): EitherT[F, BlockHistoryError, Receipt] =
    ipfs
      .upload(txs)
      .map(Receipt(height, _))
      .leftMap(se => TxsUploadingError(height, txs.size, se): BlockHistoryError)

  // TODO write docs; why do we need height here?
  private def uploadManifest(height: Long, manifest: BlockManifest)(
    implicit log: Log[F]
  ): EitherT[F, BlockHistoryError, Receipt] =
    ipfs
      .upload(manifest.jsonBytes())
      .map(Receipt(height, _))
      .leftMap(se => ManifestUploadingError(height, se): BlockHistoryError)
}
