package fluence.btree.server.integration

import fluence.btree.client.core.PutDetails
import fluence.btree.client.merkle.{ MerklePath, MerkleRootCalculator }
import fluence.btree.client.network.BTreeRpc
import fluence.btree.client.network.BTreeRpc.{ GetCallbacks, PutCallbacks }
import fluence.btree.client.{ Bytes, Key, Value }
import fluence.btree.server.MerkleBTree
import fluence.btree.server.core.{ BranchNode, GetCommand, LeafNode, PutCommand }
import fluence.hash.CryptoHasher
import monix.eval.Task
import monix.execution.atomic.Atomic

class LocalBTreeRpc(bTree: MerkleBTree, hasher: CryptoHasher[Bytes, Bytes]) extends BTreeRpc[Task] {

  override def get(getCallbacks: GetCallbacks[Task]): Task[Option[Value]] = {

    val cmd = createGetCmd(getCallbacks)

    bTree.get(cmd)
      .map(_ ⇒ cmd.getResult)
  }

  private def createGetCmd(getCallbacks: GetCallbacks[Task]) = {
    new GetCommand[Task, Key, Value] {

      private val callback: Atomic[GetCallbacks[Task]] = Atomic(getCallbacks)
      private val result: Atomic[Option[Value]] = Atomic(Option.empty[Value])

      override def nextChildIndex(branch: BranchNode[Key, _]): Task[Int] = {
        callback()
          .nextChild(branch.keys, branch.childsChecksums)
          .flatMap {
            case (newCallbacks, idx: Int) ⇒
              callback.set(newCallbacks)
              Task(idx)
          }
      }

      override def submitLeaf(leaf: Option[LeafNode[Key, Value]]): Task[Unit] = {
        val (keys, values) =
          leaf.map(l ⇒ (l.keys, l.values))
            .getOrElse((Array.empty[Key], Array.empty[Value]))
        callback().submitLeaf(keys, values)
          .map(res ⇒ result.set(res))
      }

      def getResult: Option[Value] =
        result.get
    }
  }

  override def put(putCallback: PutCallbacks[Task]): Task[Option[Value]] = {

    val cmd = createPutCmd(putCallback)

    bTree.put(cmd)
      .map(_ ⇒ cmd.getResult)
  }

  private def createPutCmd(putCallback: PutCallbacks[Task]) = {
    new PutCommand[Task, Key, Value] {

      private val callback: Atomic[PutCallbacks[Task]] = Atomic(putCallback)
      private val result: Atomic[Option[Value]] = Atomic(Option.empty[Value])
      private val mRootCalculator = MerkleRootCalculator(hasher)

      override def nextChildIndex(branch: BranchNode[Key, _]): Task[Int] = {
        callback()
          .nextChild(branch.keys, branch.childsChecksums)
          .flatMap {
            case (newCallbacks, idx: Int) ⇒
              callback.set(newCallbacks)
              Task(idx)
          }
      }

      override def putDetails(leaf: Option[LeafNode[Key, Value]]): Task[PutDetails] = {
        val (keys, values) =
          leaf.map(l ⇒ (l.keys, l.values))
            .getOrElse((Array.empty[Key], Array.empty[Value]))
        callback().submitLeaf(keys, values)
          .map {
            case (newCallbacks, putDetails) ⇒
              callback.set(newCallbacks)
              putDetails
          }
      }

      override def verifyChanges(merklePath: MerklePath, wasSplitting: Boolean): Task[Unit] = {
        callback()
          .verifyChanges(mRootCalculator.calcMerkleRoot(merklePath), wasSplitting)
          .flatMap(_.changesStored())
          .map(res ⇒ result.set(res))
      }

      def getResult: Option[Value] =
        result.get
    }
  }
}

