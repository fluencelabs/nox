package fluence.kad

import cats.data.StateT
import monix.eval.{ MVar, Task }

object RunOnMVar {
  /**
   * Runs a state modification on state V enclosed within MVar, and updates read model before return
   *
   * @param mvar State container
   * @param mod State modifier
   * @param updateRead Callback to update read model
   * @tparam T Return type
   * @tparam V State type
   * @return mod call response
   */
  def runOnMVar[T, V](mvar: MVar[V], mod: StateT[Task, V, T], updateRead: V ⇒ Unit): Task[T] =
    mvar.take.flatMap { init ⇒
      // Run modification
      mod.run(init).onErrorHandleWith { err ⇒
        // In case modification failed, write initial value back to MVar
        mvar.put(init).flatMap(_ ⇒ Task.raiseError(err))
      }
    }.flatMap {
      case (updated, value) ⇒
        // Update read and write states
        updateRead(updated)
        mvar.put(updated).map(_ ⇒ value)
    }
}
