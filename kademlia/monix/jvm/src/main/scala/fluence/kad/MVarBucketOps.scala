package fluence.kad

import monix.eval.Task

object MVarBucketOps {
  /**
   * Builds and returns a default implementation for WriteOps with Task effect
   *
   * @param maxBucketSize Maximum number of nodes in a bucket
   */
  def task[C](maxBucketSize: Int): Bucket.WriteOps[Task, C] = new TrieMapMVarBucketOps[C](maxBucketSize)
}
