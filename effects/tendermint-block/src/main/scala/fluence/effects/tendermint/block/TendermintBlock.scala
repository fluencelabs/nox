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

import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.errors.TendermintBlockError
import fluence.effects.tendermint.block.errors.ValidationError._
import fluence.effects.tendermint.block.protobuf.ProtobufJson
import scodec.bits.ByteVector

object TendermintBlock {

  def apply(json: String): Either[TendermintBlockError, TendermintBlock] = {
    ProtobufJson.block(json).map(TendermintBlock(_))
  }
}

/**
 * Representation of Tendermint's block, implements validation.
 * Can be deserialized directly from Tendermint's RPC.
 *
 * @param block Block DTO
 */
case class TendermintBlock(block: Block) {

  /**
   * Validates correctness of Merkle hashes for:
   *  1) transaction list for this block,
   *  2) last commit (votes for previous block)
   *
   * Doesn't verify any signatures; to be used to prove block correctness & txs inclusion
   *
   * @return Either a validation error, or Unit on success
   */
  def validateHashes(): Either[ValidationError, Unit] = {
    def validateOr(expected: String, actual: String, err: (String, String) => ValidationError) =
      Either.cond(expected == actual, (), err(expected, actual))

    for {
      _ <- validateOr(ByteVector(block.dataHash()).toHex, block.header.data_hash.toHex, InvalidDataHash)
      _ <- validateOr(ByteVector(block.lastCommitHash()).toHex, block.header.last_commit_hash.toHex, InvalidCommitHash)
    } yield ()
  }
}
