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

package fluence.effects.tendermint.block

import io.circe.Decoder
import io.circe.generic.semiauto.deriveDecoder
import proto3.tendermint.{BlockID, Vote}

object Commit {
  import Block._
  import Header._

  implicit val headerDecoder: Decoder[Commit] = deriveDecoder[Commit]
}

case class Commit(
  block_id: BlockID,
  precommits: List[Option[Vote]],
)
