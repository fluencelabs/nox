package fluence.btree.server.integration

import fluence.btree.client.core.PutDetails
import fluence.btree.client.merkle.{ MerklePath, MerkleRootCalculator }
import fluence.btree.client.protocol.BTreeRpc
import fluence.btree.client.protocol.BTreeRpc.{ GetCallbacks, PutCallbacks }
import fluence.btree.client.{ Bytes, Key, Value }
import fluence.btree.server.MerkleBTree
import fluence.btree.server.core.{ BranchNode, GetCommand, LeafNode, PutCommand }
import fluence.hash.CryptoHasher
import monix.eval.Task

/**
 * Local bridge between BTree client and server. Used for integration test [[IntegrationMerkleBTreeSpec]]
 */
class LocalBTreeRpc(bTree: MerkleBTree, hasher: CryptoHasher[Bytes, Bytes]) extends BTreeRpc[Task] {

  override def get(getCallbacks: GetCallbacks[Task]): Task[Unit] = {
    bTree.get(createGetCmd(getCallbacks))
  }

  private def createGetCmd(getCallbacks: GetCallbacks[Task]) = {
    new GetCommand[Task, Key, Value] {

      override def nextChildIndex(branch: BranchNode[Key, _]): Task[Int] =
        getCallbacks
          .nextChildIndex(branch.keys, branch.childsChecksums)

      override def submitLeaf(leaf: Option[LeafNode[Key, Value]]): Task[Unit] = {
        val (keys, values) =
          leaf.map(l ⇒ (l.keys, l.values))
            .getOrElse((Array.empty[Key], Array.empty[Value]))

        getCallbacks.submitLeaf(keys, values)
      }

    }
  }

  override def put(putCallback: PutCallbacks[Task]): Task[Unit] =
    bTree.put(createPutCmd(putCallback))

  private def createPutCmd(putCallback: PutCallbacks[Task]) = {
    new PutCommand[Task, Key, Value] {

      private val mRootCalculator = MerkleRootCalculator(hasher)

      override def nextChildIndex(branch: BranchNode[Key, _]): Task[Int] =
        putCallback
          .nextChildIndex(branch.keys, branch.childsChecksums)

      override def putDetails(leaf: Option[LeafNode[Key, Value]]): Task[PutDetails] = {
        val (keys, values) =
          leaf.map(l ⇒ (l.keys, l.values))
            .getOrElse((Array.empty[Key], Array.empty[Value]))

        putCallback.putDetails(keys, values)
      }

      override def verifyChanges(merklePath: MerklePath, wasSplitting: Boolean): Task[Unit] =
        putCallback
          .verifyChanges(mRootCalculator.calcMerkleRoot(merklePath), wasSplitting)
          .flatMap { _ ⇒ println("stored"); putCallback.changesStored() }

    }
  }
}

