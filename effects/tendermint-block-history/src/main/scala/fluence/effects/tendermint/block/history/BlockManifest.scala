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

import fluence.effects.tendermint.block.data.Header
import proto3.tendermint.Vote
import scodec.bits.ByteVector

case class BlockManifest(
  // TODO: Why do we need vmHash here? Wont header.appHash suffice? It's could be tricky to retrieve vmhash from the Worker
  // TODO: I guess I'll omit it for now
  vmHash: ByteVector,
  previousManifestReceipt: Option[Receipt],
  txsReceipt: Option[Receipt],
  header: Header,
  votes: List[Vote]
) {
  def bytes(): ByteVector = ???
}
