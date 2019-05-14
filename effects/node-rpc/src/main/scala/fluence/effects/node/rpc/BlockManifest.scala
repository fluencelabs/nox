package fluence.effects.node.rpc

import scodec.bits.ByteVector


case class BlockManifest() {
  def bytes(): ByteVector = ???
}

object BlockManifest {
  val Empty = BlockManifest()
}