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

import cats.instances.option._
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.monad._
import cats.instances.option._
import cats.{Applicative, Monad, Traverse}
import fluence.effects.tendermint.block.data.Block
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.{Decoder, Encoder}
import scodec.bits.ByteVector

import scala.language.higherKinds

case class Receipt() {
  // TODO: serialize to JSON, and get bytes
  def bytes(): ByteVector = ???
}

object Receipt {
  implicit val dec: Decoder[Receipt] = deriveDecoder[Receipt]
  implicit val enc: Encoder[Receipt] = deriveEncoder[Receipt]
}

// TODO: Move that class to a separate package? Validator could use that for downloading
// TODO: Pass IPFS here
case class BlockHistory[F[_]: Monad]() {

  def upload(block: Block, vmHash: ByteVector, previousManifestReceipt: Option[Receipt]): F[Receipt] = {
    val txs = block.data.txs.map(_.map(_.bv))
    val votes = block.last_commit.precommits.flatten
    for {
      // TODO: what to return on txs=None?
      txsReceipt <- Traverse[Option].sequence(txs.map(uploadTxs))
      manifest = BlockManifest(vmHash, previousManifestReceipt, txsReceipt, block.header, votes)
      receipt <- uploadManifest(manifest)
    } yield receipt
  }

  private def uploadTxs(txs: List[ByteVector]): F[Receipt] = ???
  private def uploadManifest(manifest: BlockManifest): F[Receipt] = ???
}
