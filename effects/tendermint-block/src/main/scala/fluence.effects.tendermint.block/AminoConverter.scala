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

import com.google.protobuf.ByteString
import proto3.tendermint.Vote
import scodec.bits.ByteVector

object AminoConverter {
  import proto3.tendermint.{Block => PBBlock, Commit => PBCommit, Data => PBData, Header => PBHeader}

  private def bs(bv: ByteVector): ByteString = ByteString.copyFrom(bv.toArray)
  private def serialize(precommits: List[Option[Vote]]): List[ByteString] =
    Amino.encode(precommits).map(ByteString.copyFrom)

  def toCommit(lc: LastCommit) = PBCommit(Some(lc.block_id), serialize(lc.precommits))

  def toAmino(h: Header): PBHeader = {
    PBHeader(
      version = h.version,
      chainId = h.chain_id,
      height = h.height,
      time = h.time,
      numTxs = h.num_txs,
      totalTxs = h.total_txs,
      lastBlockId = h.last_block_id,
      lastCommitHash = bs(h.last_commit_hash),
      dataHash = bs(h.data_hash),
      validatorsHash = bs(h.validators_hash),
      nextValidatorsHash = bs(h.next_validators_hash),
      consensusHash = bs(h.consensus_hash),
      appHash = bs(h.app_hash),
      lastResultsHash = bs(h.last_results_hash),
      evidenceHash = bs(h.evidence_hash),
      proposerAddress = bs(h.proposer_address),
    )
  }

  def toAmino(d: Data): PBData = {
    PBData(d.txs.map(bv64 => bs(bv64.bv)))
  }

  def toAmino(b: Block): PBBlock = {
    val header = toAmino(b.header)
    val data = toAmino(b.data)

    PBBlock(
      header = Some(header),
      data = Some(data),
      evidence = None,
      lastCommit = Some(toCommit(b.last_commit)),
    )
  }
}
