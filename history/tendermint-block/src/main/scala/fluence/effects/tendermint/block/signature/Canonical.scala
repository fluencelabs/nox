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

package fluence.effects.tendermint.block.signature

import proto3.tendermint._

/**
 * Converters of protobuf messages to a Tendermint's "canonical" form
 *
 * Canonical means that this structure all and only needed information to be signed.
 * In other words, it's just a fancy word for "ready to be signed"
 */
private[block] object Canonical {

  private def partSetHeader(header: PartSetHeader): CanonicalPartSetHeader = {
    CanonicalPartSetHeader(header.hash, header.total)
  }

  private def blockID(blockID: BlockID): CanonicalBlockID = {
    CanonicalBlockID(blockID.hash, blockID.parts.map(Canonical.partSetHeader))
  }

  def vote(vote: Vote, chainID: String): CanonicalVote = {
    val blockID = vote.blockId.map(Canonical.blockID)
    CanonicalVote(vote.`type`, vote.height, vote.round, blockID, vote.timestamp, chainID)
  }
}
