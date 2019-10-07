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

import cats.effect.concurrent.Ref
import cats.syntax.flatMap._
import fluence.bp.api.BlockStream
import fluence.effects.tendermint.block.data.Block
import fluence.log.Log

import scala.language.higherKinds

case class ControlledBlockStream[F[_]](blocks: Seq[Block], state: Ref[F, UploadingState])
    extends BlockStream[F, Block] {
  override def freshBlocks(implicit log: Log[F]): fs2.Stream[F, Block] =
    throw new NotImplementedError("def freshBlocks")

  override def blocksSince(height: Long)(implicit log: Log[F]): fs2.Stream[F, Block] =
    fs2.Stream.eval(state.update(_.subscribe(height))) >> fs2.Stream.emits(blocks)
}
