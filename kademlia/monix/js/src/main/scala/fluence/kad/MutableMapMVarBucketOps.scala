package fluence.kad

import cats.data.StateT
import monix.eval.{ MVar, Task }

/**
 * Implementation of Bucket.WriteOps, based on MVar and mutable map -- good for JS
 *
 * @param maxBucketSize Max number of nodes in each bucket
 * @tparam C Node contacts
 */
class MutableMapMVarBucketOps[C](maxBucketSize: Int) extends Bucket.WriteOps[Task, C] {
  private val writeState = collection.mutable.Map.empty[Int, MVar[Bucket[C]]]
  private val readState = collection.mutable.Map.empty[Int, Bucket[C]]

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
