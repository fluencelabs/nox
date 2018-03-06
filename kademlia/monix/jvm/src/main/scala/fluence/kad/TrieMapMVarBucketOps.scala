package fluence.kad

import cats.data.StateT
import monix.eval.{ MVar, Task }

import scala.collection.concurrent.TrieMap

/**
 * Implementation of Bucket.WriteOps, based on MVar and TrieMap -- available only on JVM
 *
 * @param maxBucketSize Max number of nodes in each bucket
 * @tparam C Node contacts
 */
class TrieMapMVarBucketOps[C](maxBucketSize: Int) extends Bucket.WriteOps[Task, C] {
  private val writeState = TrieMap.empty[Int, MVar[Bucket[C]]]
  private val readState = TrieMap.empty[Int, Bucket[C]]

  import RunOnMVar.runOnMVar

  override protected def run[T](bucketId: Int, mod: StateT[Task, Bucket[C], T]): Task[T] =
    runOnMVar(
      writeState.getOrElseUpdate(bucketId, MVar(read(bucketId))),
      mod,
      readState.update(bucketId, _: Bucket[C])
    )

  override def read(bucketId: Int): Bucket[C] =
    readState.getOrElseUpdate(bucketId, Bucket[C](maxBucketSize))
}
