/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

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
