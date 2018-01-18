package fluence.kad.testkit

import cats.data.StateT
import fluence.kad.Bucket
import monix.eval.Coeval

import scala.language.higherKinds

class TestBucketOps[C](maxBucketSize: Int) extends Bucket.WriteOps[Coeval, C] {
  self ⇒

  private val buckets = collection.mutable.Map.empty[Int, Bucket[C]]
  private val locks = collection.mutable.Map.empty[Int, Boolean].withDefaultValue(false)

  override protected def run[T](bucketId: Int, mod: StateT[Coeval, Bucket[C], T]) = {
    require(!locks(bucketId), s"Bucket $bucketId must be not locked")
    locks(bucketId) = true
    //println(s"Bucket $bucketId locked")

    mod.run(read(bucketId)).map {
      case (b, v) ⇒
        buckets(bucketId) = b
        v
    }.doOnFinish{ _ ⇒
      //println(s"Bucket $bucketId unlocked")
      Coeval.now(locks.update(bucketId, false))
    }
  }

  override def read(bucketId: Int): Bucket[C] =
    buckets.getOrElseUpdate(bucketId, Bucket(maxBucketSize))

}
