package fluence.kad.testkit

import cats.data.StateT
import fluence.kad.Siblings
import fluence.kad.protocol.Key
import monix.eval.Coeval
import monix.execution.atomic.{ Atomic, AtomicBoolean }

import scala.language.higherKinds

class TestSiblingOps[C](nodeId: Key, maxSiblingsSize: Int) extends Siblings.WriteOps[Coeval, C] {
  self ⇒

  private val state = Atomic(Siblings[C](nodeId, maxSiblingsSize))
  private val lock = AtomicBoolean(false)

  override protected def run[T](mod: StateT[Coeval, Siblings[C], T]): Coeval[T] = {
    Coeval {
      require(lock.flip(true), "Siblings must be unlocked")
      lock.set(true)
    }.flatMap(_ ⇒
      mod.run(read).map {
        case (s, v) ⇒
          state.set(s)
          v
      }.doOnFinish(_ ⇒ Coeval.now(lock.flip(false))))
  }

  override def read: Siblings[C] =
    state.get

}
