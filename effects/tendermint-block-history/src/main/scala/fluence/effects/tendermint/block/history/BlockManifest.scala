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
