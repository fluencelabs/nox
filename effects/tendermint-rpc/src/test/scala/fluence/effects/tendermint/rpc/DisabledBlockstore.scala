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

package fluence.effects.tendermint.rpc

import cats.{Applicative, Functor}
import cats.data.EitherT
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.db.{Blockstore, BlockstoreError, DbNotFound}

import scala.language.higherKinds

class DisabledBlockstore[F[_]: Applicative] extends Blockstore[F] {

  override def getBlock(height: Long): EitherT[F, BlockstoreError, Block] =
    EitherT.leftT[F, Block](DbNotFound("Blockstore is disabled in tests"): BlockstoreError)

  override def getStorageHeight: EitherT[F, BlockstoreError, Long] =
    EitherT.leftT[F, Long](DbNotFound("Blockstore is disabled in tests"): BlockstoreError)
}

object DisabledBlockstore {
  def apply[F[_]: Applicative](): DisabledBlockstore[F] = new DisabledBlockstore()
}
