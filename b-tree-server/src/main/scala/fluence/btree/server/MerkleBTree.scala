package fluence.btree.server

import java.nio.ByteBuffer
import java.util.concurrent.atomic.AtomicReference

import cats.syntax.show._
import fluence.btree.client.merkle.{ MerklePath, NodeProof }
import fluence.btree.client.{ Key, ReadResults, Value, WriteResults }
import fluence.btree.server.binary.BTreeBinaryStore
import fluence.node.binary.kryo.KryoCodecs
import fluence.btree.server.core.TreePath.PathElem
import fluence.btree.server.core.{ NodeWithId, _ }
import fluence.node.storage.rocksdb.RocksDbStore
import monix.eval.{ Task, TaskSemaphore }
import monix.execution.atomic.{ AtomicInt, AtomicLong }
import org.slf4j.LoggerFactory

import scala.collection.Searching.{ Found, InsertionPoint, SearchResult }

/**
 * This class implements a search tree, which allows to run queries over encrypted data. This code based on research paper:
 * '''Popa R.A.,Li F.H., Zeldovich N. 'An ideal-security protocol for order-preserving encoding.' 2013 '''
 *
 * In its essence this tree is a hybrid of B+Tree [[https://en.wikipedia.org/wiki/B%2B_tree]] and MerkleTree
 * [[https://en.wikipedia.org/wiki/Merkle_tree]] data structures.
 *
 * This ''B+tree'' is an N-ary tree with a number of children per node ranging between ''MinDegree'' and ''MaxDegree''.
 * A tree consists of a root, internal branch nodes and leaves. The root may be either a leaf or a node with two or
 * more children. Copies of the some keys are stored in the internal nodes (for efficient searching); keys and
 * records are stored in leaves. Tree is kept balanced by requiring that all leaf nodes are at the same depth.
 * This depth will increase slowly as elements are added to the tree. Depth increases only when the root is being splitted.
 *
 * Note that the tree provides only algorithms (i.e., functions) to search, insert and delete elements with
 * corresponding Merkle proofs. Tree nodes are actually stored externally using the [[BTreeStore]] to make the tree
 * maximally pluggable and seamlessly switch between in memory, on disk, or maybe, over network storages. Key comparison
 * operations in the tree are also pluggable and are provided by the [[TreeRouter]] implementations, which helps to
 * impose an order over for example encrypted nodes data.
 *
 * @param conf    Config for this tree
 * @param store   BTree persistence store for persisting tree nodes
 * @param router  Finds key position in tree node keys. Comparisons are performed remotely on the client
 * @param nodeOps Operations performed on nodes
 */
class MerkleBTree private[server] (
    conf: MerkleBTreeConfig,
    store: BTreeStore[NodeId, Node, Task],
    router: TreeRouter[Key, SearchResult, Task],
    nodeOps: NodeOps
) {

  import MerkleBTree._
  import MerkleBTreeShow._
  import nodeOps._

  /** max of node size */
  private val MaxDegree = conf.arity
  /** min of node size except root */
  private val MinDegree = conf.alpha * conf.arity
  /* tree root id is constant, it always point to root node. */
  private val RootId = 0L

  /* number of tree levels */
  private val depth = AtomicInt(0)
  /* generates new node id when splitting node happened */
  private val idNodeCounter = AtomicLong(RootId)
  /* mutex for single-thread access to a tree */
  private val globalMutex = TaskSemaphore(1)

  /* Public methods */

  /**
   * === GET ===
   *
   * We are looking for a specified key in this B+Tree.
   * Starting from the root, we are looking for the leaf which may contain the ''key'' using [[TreeRouter]] for
   * comparing keys. At each node, we figure out which internal pointer we should follow. When we go down we are
   * storing merkle path for verifying searched results. Get have O(log,,arity,,n) algorithmic complexity.
   *
   * @param key Search key
   * @return Task with result [[ReadResults]] contains search ''key'', searched ''value'' and merkle path for proofing.
   */
  def get(key: Key): Task[ReadResults] = {
    globalMutex.greenLight(getRoot.flatMap(root ⇒ getForRoot(root, key)))
  }

  /**
   *  === PUT ===
   *
   * We are looking for a specified key in this B+Tree.
   * Starting from the root, we are looking for the leaf which may contain the ''key'' using [[TreeRouter]] for
   * comparing keys. At each node, we figure out which internal pointer we should follow. When we go down the tree
   * we put each visited node to ''Trail''(see [[TreePath]]) in the same order. Trail is just a array of all visited
   * nodes from root to leaf. When we found slot for insertion we do all tree transformation in logical copy of sector
   * of tree; actually ''Trail'' is this copy - copy of visited nodes that will be changed after insert. We insert new
   * ''key'' and ''value'' and split leaf if leaf is full and we split leaf parent if parent is filled and so on to
   * the root. Also after changing leaf we should re-calculate merkle root and update checksums of all visited nodes.
   * Absolutely all tree transformations are performed on copies and do not change the tree. When all transformation
   * in logical state ended we commit changes (see method 'commitNewState').
   * Put have O(log,,arity,,n) algorithmic complexity.
   *
   * @param key   Search key (will be stored as new key)
   * @param value Value for putting (will be stored as new value)
   * @return Task with result [[WriteResults]] contains search ''key'', inserted ''value'', old merkle path for
   *         proofing before inserting and new merkle path after inserting for re-calculating new merkle root on the client
   */
  def put(key: Key, value: Value): Task[WriteResults] = {
    globalMutex.greenLight(getRoot.flatMap(root ⇒ putForRoot(root, key, value)))
  }

  def delete(key: Key): Task[ReadResults] = ???

  def getDepth: Int = depth.get

  def getMerkleRoot: Task[Hash] = getRoot.map(_.checksum)

  /* Private methods */

  private def isEmpty(node: Node): Boolean = node.size == 0

  private def hasOverflow(node: Node): Boolean = node.size > MaxDegree

  private[btree] def getRoot: Task[Node] = {
    Task(getDepth).flatMap {
      case 0 ⇒
        val emptyLeaf = LeafNode(Array.empty[Key], Array.empty[Value], Array.empty[Hash], 0, Array.emptyByteArray)
        commitNewState(PutTask(Seq(NodeWithId(RootId, emptyLeaf))))
          .map(_ ⇒ emptyLeaf)
      case _ ⇒
        store.get(RootId)

    }
  }

  /* GET */

  /*** Entry point for any Get operations. */
  private def getForRoot(root: Node, key: Key): Task[ReadResults] = {
    log.debug(s"(${key.show}) Get starts")
    getForNode(root, key, MerklePath.empty)
  }

  private def getForNode(root: Node, key: Key, mPath: MerklePath): Task[ReadResults] = {
    if (isEmpty(root)) {
      return Task(ReadResults(key, None, mPath))
    }

    // TODO: fix unchecked error
    root match {
      case leaf: Leaf ⇒
        getForLeaf(leaf, key, mPath)
      case branch: Branch ⇒
        getForBranch(branch, key, mPath)
    }
  }

  /** '''Method makes remote call!''' This method makes step down the tree and updates merkle path. */
  private def getForBranch(branch: Branch, key: Key, mPath: MerklePath): Task[ReadResults] = {
    log.debug(s"(${key.show}) Get for branch=${branch.show}, path=$mPath")

    searchChild(branch, key)
      .flatMap {
        case (idx, child) ⇒
          val newPath = mPath.add(branch.toProof(idx))
          getForNode(child, key, newPath)
      }
  }

  /**
   * '''Method makes remote call!'''.
   * Searches ''key'' into this leaf. Also create [[MerklePath]] as merkle proof and compose [[ReadResults]].
   * This is the terminal method.
   *
   * @param leaf   Leaf for searching
   * @param key    Search key
   * @param mPath  Merkle path for prev visited nodes
   */
  private def getForLeaf(leaf: Leaf, key: Key, mPath: MerklePath): Task[ReadResults] = {
    log.debug(s"(${key.show}) Get for leaf=${leaf.show}, mPath=$mPath")

    router
      .indexOf(key, leaf.keys)
      .map {
        case Found(idx) ⇒
          val newPath = mPath.add(leaf.toProof(idx))
          ReadResults(key, Some(leaf.values(idx)), newPath)
        case InsertionPoint(idx) ⇒
          // if searched element not found returns -1 as in substitutionIdx
          val newPath = mPath.add(leaf.toProof(-1))
          ReadResults(key, None, newPath)
      }
  }

  /* PUT */

  /*** Entry point for any put operations. */
  private def putForRoot(root: Node, key: Key, value: Value): Task[WriteResults] = {
    log.debug(s"(${key.show} - ${value.show}) Put starts")

    // if root is empty don't need to finding slot for putting
    if (isEmpty(root)) {
      log.debug(s"(${key.show} - ${value.show}) Root is empty")

      val newLeaf = createLeaf(key, value)
      val oldStateProof = MerklePath.empty
      val newStateProof = createPath(newLeaf, 0, TreePath.empty)

      commitNewState(PutTask(nodesToSave = Seq(NodeWithId(RootId, newLeaf)), increaseDepth = true))
        .map(_ ⇒ WriteResults(key, value, oldStateProof, newStateProof))

    } else {
      putForNode(key, value, RootId, root, TreePath.empty)
    }
  }

  private def putForNode(key: Key, value: Value, id: NodeId, node: Node, trail: Trail): Task[WriteResults] = {
    // TODO: fix unchecked error
    node match {
      case leaf: Leaf ⇒
        putForLeaf(key, value, id, leaf, trail)
      case branch: Branch ⇒
        putForBranch(key, value, id, branch, trail)
    }
  }

  /**
   * '''Method makes remote call!'''.
   * This method finds and fetches next child, makes step down the tree and updates trail.
   *
   * @param key    New key for updating
   * @param value  New value for updating
   * @param branchId Id of walk-through branch node
   * @param branch   Walk-through branch node
   * @param trail  The path traversed from the root
   */
  private def putForBranch(key: Key, value: Value, branchId: NodeId, branch: Branch, trail: Trail): Task[WriteResults] = {
    log.debug(s"(${key.show} - ${value.show}) Put to branch=${branch.show}, id=$branchId")

    router
      .indexOf(key, branch.keys)
      .flatMap(searchResult ⇒ {
        val searchedIdx = searchResult.insertionPoint
        val childId = branch.children(searchedIdx)
        store.get(childId)
          .flatMap { child ⇒
            val newTrail = trail.addBranch(branchId, branch, searchedIdx)
            putForNode(key, value, childId, child, newTrail)
          }
      })
  }

  /**
   * '''Method makes remote call!'''.
   * Puts new ''key'' and ''value'' to this leaf.
   * Also makes all tree transformation (rebalancing, persisting to store).
   * Finally creates ''oldStateProof'', ''newStateProof'' and compose [[WriteResults]].
   * This is the terminal method.
   *
   * @param key    New key for updating
   * @param value  New value for updating
   * @param leafId Id of updatable leaf
   * @param leaf   Updatable Leaf
   * @param trail  The path traversed from the root
   */
  private def putForLeaf(
    key: Key,
    value: Value,
    leafId: NodeId,
    leaf: Leaf,
    trail: Trail
  ): Task[WriteResults] = {
    log.debug(s"(${key.show} - ${value.show}) Put to leaf=${leaf.show}, id=$leafId")

    router
      .indexOf(key, leaf.keys)
      .flatMap(searchResult ⇒ {
        val oldStateProof = createPath(leaf, searchResult.insertionPoint, trail)
        val updatedLeaf = updateLeaf(key, value, leaf, searchResult)
        // makes all transformations over the copy of tree
        val (newStateProof, putTask) = logicalPut(leafId, updatedLeaf, searchResult.insertionPoint, trail)

        // persist all changes
        commitNewState(putTask)
          .map(_ ⇒ WriteResults(key, value, oldStateProof, newStateProof))
      })
  }

  /**
   * This method do all mutation operations over the tree in memory without changing tree state
   * and composes merkle path for new tree state. It inserts new value to leaf, and do tree rebalancing if it needed.
   * All changes occur over copies of the visited nodes and actually don't change the tree.
   *
   * @param leafId            Id of leaf that was updated
   * @param newLeaf           Leaf that was updated with new key and value
   * @param searchedValueIdx Insertion index of a new value
   * @param trail             The path traversed from the root to a leaf with all visited tree nodes.
   * @return Tuple with [[MerklePath]] for tree after updating and [[PutTask]] for persisting changes
   */
  private def logicalPut(
    leafId: NodeId,
    newLeaf: Leaf,
    searchedValueIdx: Int,
    trail: Trail
  ): (MerklePath, PutTask) = {
    log.debug(s"Logic put for leafId=$leafId, leaf=$newLeaf, trail=$trail")

    /**
     * Just a state for each recursive operation of ''logicalPut''.
     *
     * @param newStateProof  Merkle path after ''put'' operation, uses for calculate new merkle root by client
     * @param updateParentFn Function-mutator that will be applied to parent of current node
     */
    case class PutCtx(
        newStateProof: MerklePath,
        updateParentFn: PathElem[NodeId, Branch] ⇒ PathElem[NodeId, Branch] = identity,
        putTask: PutTask
    )

    /**
     * If leaf isn't overflowed
     * - updates leaf checksum into parent node and put leaf and it's parent to ''nodesToSave'' into [[PutTask]].
     *
     * If it's overflowed
     * - splits leaf into two, adds left leaf to parent as new child and update right leaf checksum into parent node.
     * - If parent ins't exist create new parent with 2 new children.
     * - Put all updated and new nodes to ''nodesToSave'' into [[PutTask]]
     *
     * @param leafId            Id of leaf that was updated
     * @param newLeaf           Leaf that was updated with new key and value
     * @param searchedValueIdx Insertion index of a new value
     * @param hasParent         True if this leaf has parent node, false - if this leaf is root
     */
    def createLeafCtx(leafId: NodeId, newLeaf: Leaf, searchedValueIdx: Int, hasParent: Boolean): PutCtx = {

      if (hasOverflow(newLeaf)) {
        log.debug(s"Do split for leafId=$leafId, leaf=${newLeaf.show}")

        val isRoot = leafId == RootId
        val (left, right) = newLeaf.split
        // get ids for new nodes
        val leftId = idNodeCounter.incrementAndGet()
        // RootId is always linked with root node and will not changed, store right node with new id if split root
        val rightId = if (isRoot) idNodeCounter.incrementAndGet() else leafId

        val isInsertToTheLeft = searchedValueIdx < left.size
        val affectedLeaf = if (isInsertToTheLeft) left else right
        val affectedLeafIdx = if (isInsertToTheLeft) searchedValueIdx else searchedValueIdx - left.size
        val merklePath = MerklePath(Array(affectedLeaf.toProof(affectedLeafIdx)))

        if (isRoot) {
          // there is no parent, root leaf was splitted
          val popUpKey = left.keys.last
          val newParent = createBranch(popUpKey, ChildRef(leftId, left.checksum), ChildRef(rightId, right.checksum))
          val affectedParentIdx = if (isInsertToTheLeft) 0 else 1
          PutCtx(
            newStateProof = MerklePath(newParent.toProof(affectedParentIdx) +: merklePath.path),
            putTask = PutTask(
              nodesToSave = Seq(NodeWithId(leftId, left), NodeWithId(rightId, right), NodeWithId(RootId, newParent)),
              increaseDepth = true  // if splitting root-leaf appears - increase depth of tree
            )
          )
        } else {
          // some regular leaf was splitted
          PutCtx(
            newStateProof = merklePath,
            updateParentFn = updateAfterChildSplitting(NodeWithId(leftId, left), NodeWithId(rightId, right), isInsertToTheLeft),
            putTask = PutTask(nodesToSave = Seq(NodeWithId(leftId, left), NodeWithId(rightId, right)))
          )
        }
      } else {
        PutCtx(
          newStateProof = MerklePath(Array(newLeaf.toProof(searchedValueIdx))),
          updateParentFn = updatedAfterChildChanging(newLeaf.checksum),
          putTask = PutTask(nodesToSave = Seq(NodeWithId(leafId, newLeaf)))
        )
      }
    }

    /**
     * Note that this method returns function that used for folding all visited branches from ''trail''.
     *
     * Returned function do as follow:
     * 
     * If branch isn't overflowed
     * - updates branch checksum into parent node and put branch and it's parent to ''nodesToSave'' into [[PutTask]].
     *
     * If it's overflowed
     * - splits branch into two, adds left branch to parent as new child and update right branch checksum into parent node.
     * - If parent ins't exist create new parent with 2 new children.
     * - Put all updated and new nodes to ''nodesToSave'' into [[PutTask]]
     */
    def createTreePathCtx: (PathElem[NodeId, Branch], PutCtx) ⇒ PutCtx = {
      case (visitedBranch, PutCtx(merklePath, updateParentFn, PutTask(nodesToSave, addToDepth))) ⇒

        val PathElem(branchId, branch, nextChildIdx) = updateParentFn(visitedBranch)

        if (hasOverflow(branch)) {
          log.debug(s"Do split for branchId=$branchId, branch=${branch.show}, nextChildIdx=$nextChildIdx ")

          val isRoot = branchId == RootId
          val (left, right) = branch.split

          val leftId = idNodeCounter.incrementAndGet()
          // RootId is always linked with root node and will not changed, store right node with new id if split root
          val rightId = if (isRoot) idNodeCounter.incrementAndGet() else branchId

          val isInsertToTheLeft = nextChildIdx < left.size
          val affectedBranch = if (isInsertToTheLeft) left else right
          val affectedBranchIdx = if (isInsertToTheLeft) nextChildIdx else nextChildIdx - left.size
          val newMerklePath = MerklePath(affectedBranch.toProof(affectedBranchIdx) +: merklePath.path)

          if (isRoot) {
            // there was no parent, root node was splitting
            val popUpKey = left.keys.last
            val newParent = createBranch(popUpKey, ChildRef(leftId, left.checksum), ChildRef(rightId, right.checksum))
            val affectedNewParentIdx = if (isInsertToTheLeft) 0 else 1

            PutCtx(
              newStateProof = MerklePath(newParent.toProof(affectedNewParentIdx) +: newMerklePath.path),
              putTask = PutTask(
                nodesToSave = nodesToSave ++ Seq(NodeWithId(leftId, left), NodeWithId(rightId, right), NodeWithId(RootId, newParent)),
                increaseDepth = true // if splitting root node appears - increase depth of the tree
              )
            )
          } else {
            // some regular leaf was splitting
            PutCtx(
              newStateProof = newMerklePath,
              updateParentFn = updateAfterChildSplitting(NodeWithId(leftId, left), NodeWithId(rightId, right), isInsertToTheLeft),
              putTask = PutTask(nodesToSave ++ Seq(NodeWithId(leftId, left), NodeWithId(rightId, right)))
            )
          }
        } else {
          PutCtx(
            newStateProof = MerklePath(branch.toProof(nextChildIdx) +: merklePath.path),
            updateParentFn = updatedAfterChildChanging(branch.checksum),
            putTask = PutTask(nodesToSave :+ NodeWithId(branchId, branch))
          )
        }
    }

    /** Returns function that update childs checksum into parent node */
    def updatedAfterChildChanging(childChecksum: Hash): PathElem[NodeId, Branch] ⇒ PathElem[NodeId, Branch] =
      visitedBranch ⇒
        visitedBranch.copy(branch = visitedBranch.branch.updateChildChecksum(childChecksum, visitedBranch.nextChildIdx))

      /**
       * This method returns function that makes two changes into the parent node:
       *  1. Inserts left node as new child before right node.
       *  2. Update checksum of changed right node.
       *
       * @param left  Left node with their id
       * @param right Right node with their id
       * @param isInsertToTheLeft Direction of further descent. True if inserted value will be update left node, false otherwise.
       * @return Function for parent updating
       */
      def updateAfterChildSplitting(
        left: NodeAndId,
        right: NodeAndId,
        isInsertToTheLeft: Boolean
      ): PathElem[NodeId, Branch] ⇒ PathElem[NodeId, Branch] = {

      case PathElem(parentId: NodeId, parentNode: Branch, nextChildIdx) ⇒
        val popUpKey = left.node.keys.last
        log.trace(s"Add child to parent node: insertedKey=${popUpKey.show}, insertedChild=$left, insIdx=$nextChildIdx")
        // updates parent node with new left node. Parent already contains right node as a child.
        // update right node checksum needed, checksum of right node was changed after splitting
        val branch = parentNode
          .insertChild(popUpKey, ChildRef(left.id, left.node.checksum), nextChildIdx)
          .updateChildChecksum(right.node.checksum, nextChildIdx + 1)

        val idxOfUpdatedChild = if (isInsertToTheLeft) nextChildIdx else nextChildIdx + 1

        PathElem(parentId, branch, idxOfUpdatedChild)
    }

    val leafPutCtx: PutCtx = createLeafCtx(leafId, newLeaf, searchedValueIdx, trail.branches.nonEmpty)
    val PutCtx(newStateProof, _, putTask) = trail.branches.foldRight(leafPutCtx)(createTreePathCtx)
    newStateProof -> putTask
  }

  /**
   * Save all changed nodes to tree store. Apply putTask to old tree state for getting new tree state.
   *
   * @param putTask Pool of changed nodes
   */
  private def commitNewState(putTask: PutTask): Task[Unit] = {
    log.debug(s"commitNewState for nodes=${putTask.nodesToSave}")
    // todo start transaction
    Task.gatherUnordered(putTask.nodesToSave.map { case NodeWithId(id, node) ⇒ saveNode(id, node) })
        .foreachL(_ ⇒ if (putTask.increaseDepth) this.depth.increment())
    // todo end transaction
  }

  /**
   * Puts new ''key'' and ''value'' to this leaf.
   * If search key was found - rewrites key and value, if key wasn't found - inserts new key and value.
   *
   * @return Updated leaf with new ''key'' and ''value''
   */
  private def updateLeaf(key: Key, value: Value, leaf: Leaf, searchResult: SearchResult): Leaf = {
    searchResult match {
      case Found(idxOfUpdate) ⇒
        // key was founded in this Leaf, update leaf with new value
        leaf.rewriteKv(key, value, idxOfUpdate)
      case InsertionPoint(indexOfInsert) ⇒
        // key wan't found in this Leaf, insert new value to the leaf
        leaf.insertKv(key, value, indexOfInsert)
    }
  }

  /* Common methods */

  /** Save specified node to tree store or locally in case when node is root. */
  private def saveNode(nodeId: NodeId, node: Node): Task[Unit] = {
    log.debug(s"Save node (id=$nodeId,node=${node.show})")
    assert(assertKeyIanAscOrder(node), s"Ascending order of keys required! Invalid node=${node.show})")
    store.put(nodeId, node)
  }

  /**
   * '''Method makes remote call!'''. Searches and returns ''child node'' of tree for specified key.
   * First of all we call remote client for getting index of child for specified key.
   * After that we gets child ''nodeId'' by this index. By ''nodeId'' we fetch ''child node'' from store.
   *
   * @param branch Branch node for searching
   * @param key  Search key
   * @return     Index of searched child and the child
   */
  private def searchChild(branch: Branch, key: Key): Task[(Int, Node)] = {
    router
      .indexOf(key, branch.keys)
      .flatMap(sr ⇒ {
        val searchedIdx = sr.insertionPoint
        val childId = branch.children(searchedIdx)
        store.get(childId).map(searchedIdx → _)
      })
  }

  /** Creates [[MerklePath]] from all visited branches from ''trail'' and specified leaf  */
  private def createPath(leaf: Leaf, searchedIdx: Int, trail: Trail): MerklePath = {
    val leafProof = leaf.toProof(searchedIdx)
    val branchesProof = trail.branches.map {
      case PathElem(_, branch, substitutionIdx) ⇒ branch.toProof(substitutionIdx)
    }
    MerklePath(branchesProof :+ leafProof)
  }

  // this method used only with enabled assertion in tests for verifying order of keys into node.
  private def assertKeyIanAscOrder(node: Node): Boolean = {
    val lt: (Key, Key) ⇒ Boolean = (x, y) ⇒ ByteBuffer.wrap(x).compareTo(ByteBuffer.wrap(y)) < 0
    node.keys.sliding(2).forall {
      case Array(prev, next) ⇒ lt(prev, next)
      case _                 ⇒ true
    }
  }

}

object MerkleBTree {

  private val log = LoggerFactory.getLogger(getClass)

  /**
   * Creates new instance of MerkleBTree.
   *
   * @param treeId        Any unique name of this tree (actually will be created RockDb instance with data folder == id)
   * @param conf          MerkleBTree config
   * @param treeRouter   Allow searching key in node keys
   */
  def apply(
    treeId: String,
    conf: MerkleBTreeConfig = MerkleBTreeConfig.read(),
    treeRouter: TreeRouter[Key, SearchResult, Task],
  ): MerkleBTree = {
    new MerkleBTree(conf, defaultStore(treeId), treeRouter, NodeOps())
  }

  /**
   * Default tree store with RockDb key-value storage under the hood.
   *
   * @param id Unique id of tree used as RockDb data folder name.
   */
  private def defaultStore(id: String): BTreeStore[Long, Node, Task] = {
    val codecs = KryoCodecs()
      .add[Key]
      .add[Array[Key]]
      .add[Value]
      .add[Array[Value]]
      .add[NodeId]
      .add[Array[NodeId]]

      .add[Int]
      .add[Node]
      .addCase(classOf[Leaf])
      .addCase(classOf[Branch])
      .build[Task]()
    import codecs._
    new BTreeBinaryStore[NodeId, Node, Task](RocksDbStore(id).get)
  }

  /**
   * Task for persisting. Contains updated node after inserting new value and rebalancing the tree.
   *
   * @param nodesToSave    Pool of changed nodes that should be persisted to tree store
   * @param increaseDepth  If root node was splitted than tree depth should be increased.
   *                          If true - tree depth will be increased in physical state, if false - depth won't changed.
   *                          Note that each put operation might increase root depth only by one.
   */
  case class PutTask(nodesToSave: Seq[NodeWithId[NodeId, Node]], increaseDepth: Boolean = false)

}
