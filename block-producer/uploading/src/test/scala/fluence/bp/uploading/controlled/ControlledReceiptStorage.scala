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
import fluence.effects.receipt.storage.{ReceiptStorage, ReceiptStorageError}
import fluence.effects.tendermint.block.history.Receipt
import fluence.log.Log

import scala.language.higherKinds

case class ControlledReceiptStorage[F[_]: Applicative](val appId: Long, storedReceipts: Seq[Receipt] = Nil)
    extends ReceiptStorage[F] {
  override def put(height: Long, receipt: Receipt)(
    implicit log: Log[F]
  ): EitherT[F, ReceiptStorageError, Unit] = EitherT.pure(())

  override def get(height: Long)(implicit log: Log[F]): EitherT[F, ReceiptStorageError, Option[Receipt]] =
    EitherT.pure(None)

  override def retrieve(from: Option[Long], to: Option[Long])(
    implicit log: Log[F]
  ): fs2.Stream[F, (Long, Receipt)] =
    fs2.Stream.emits(storedReceipts.map(r => r.height -> r))
}
