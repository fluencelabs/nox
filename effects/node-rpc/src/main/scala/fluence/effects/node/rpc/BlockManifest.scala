package fluence.effects.node.rpc

import fluence.effects.tendermint.block.data.Header
import proto3.tendermint.Vote
import scodec.bits.ByteVector

case class BlockManifest(
  vmHash: ByteVector,
  previousManifestReceipt: Option[Receipt],
  txsReceipt: Option[Receipt],
  header: Header,
  votes: List[Vote]
) {
  def bytes(): ByteVector = ???
}
