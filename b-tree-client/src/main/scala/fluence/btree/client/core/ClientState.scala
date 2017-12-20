package fluence.btree.client.core

import fluence.btree.client.Bytes

/**
 * Global state of BTree client.
 *
 * @param merkleRoot Actual merkle root.
 */
case class ClientState(merkleRoot: Bytes)
