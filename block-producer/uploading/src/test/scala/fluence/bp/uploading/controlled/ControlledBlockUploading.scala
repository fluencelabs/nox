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

package fluence.bp.uploading.controlled

import cats.Applicative
import cats.data.EitherT
import cats.effect.concurrent.Ref
import cats.effect.{ConcurrentEffect, ContextShift, Resource, Sync, Timer}
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.bp.uploading.BlockUploading
import fluence.effects.castore.StoreError
import fluence.effects.ipfs.{IpfsData, IpfsUploader}
import fluence.effects.sttp.SttpStreamEffect
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.{BlockManifest, Receipt}
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.statemachine.api.command.ReceiptBus
import fluence.statemachine.api.data.BlockReceipt
import scodec.bits.ByteVector

import scala.language.higherKinds

object ControlledBlockUploading {

  private def makeState[F[_]: Sync] = Resource.liftF(Ref.of[F, UploadingState](UploadingState()))

  def makeAndStart[F[_]: Timer: ConcurrentEffect: ContextShift: SttpStreamEffect: Log](
    blocks: Seq[Block] = Nil,
    storedReceipts: Seq[Receipt] = Nil
  )(implicit backoff: Backoff[EffectError]): Resource[F, Ref[F, UploadingState]] = makeState >>= { state ⇒
    val appId = 1L

    val ipfs = new IpfsUploader[F] {
      override def upload[A: IpfsData](data: A)(implicit log: Log[F]): EitherT[F, StoreError, ByteVector] = {
        EitherT.liftF(state.update(_.upload(data)).map(_ => ByteVector.empty))
      }
    }

    val receiptBus = new ReceiptBus[F] {
      override def sendBlockReceipt(receipt: BlockReceipt)(implicit log: Log[F]): EitherT[F, EffectError, Unit] =
        EitherT.liftF(state.update(_.receipt(receipt)).void)

      override def getVmHash(height: Long)(implicit log: Log[F]): EitherT[F, EffectError, ByteVector] =
        EitherT.liftF(state.update(_.vmHash(height)).map(_ => ByteVector.empty))
    }

    val blockStream = ControlledBlockStream(blocks, state)

    val receiptStorage = ControlledReceiptStorage(appId, storedReceipts)
    val onUploaded: (BlockManifest, Receipt) ⇒ F[Unit] = (_, _) => Applicative[F].unit

    BlockUploading(enabled = true, ipfs)
      .flatMap(
        _.start(appId, receiptStorage, blockStream, receiptBus, onUploaded)
      )
      .as(state)
  }
}
