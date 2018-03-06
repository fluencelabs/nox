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

import scala.collection.concurrent.TrieMap

//todo move this to some utility module, replace all similar caches with MVarCache
//todo we always return non-default parameters, prove it on types
/**
 * WIP
 */
class MVarMapCache[K, V](default: V) {
  //todo maybe store Option[V] and add MVar(None) for default as values, on `get` return None if None in MVar (flatten)
  private val writeState = TrieMap.empty[K, MVar[V]]
  private val readState = TrieMap.empty[K, V]

  import MVarMapCache.runOnMVar

  def update(key: K, value: V): Task[Unit] =
    runOnMVar(
      writeState.getOrElseUpdate(key, MVar(value)),
      StateT.set(value),
      readState.update(key, _: V)
    )

  def get(key: K): Option[V] =
    readState.get(key)

  def getOrAdd(key: K, value: V): Task[V] =
    runOnMVar(
      writeState.getOrElseUpdate(key, MVar(value)),
      StateT.get[Task, V],
      readState.update(key, _: V)
    )

  def getOrAddF(key: K, value: ⇒ Task[V]): Task[V] =
    runOnMVar(
      writeState.getOrElseUpdate(key, MVar(default)),
      StateT.modifyF[Task, V](a ⇒ if (a != default) Task.pure(a) else value).get,
      readState.update(key, _: V)
    )

  def modify(key: K, modify: V ⇒ V): Task[Boolean] =
    writeState.get(key) match {
      case None ⇒ Task.pure(false)
      case Some(value) ⇒
        runOnMVar(
          value,
          StateT.modify[Task, V](modify).map(_ ⇒ true),
          readState.update(key, _: V)
        )
    }

  def modifyValueOrDefault(key: K, modify: V ⇒ V): Task[Boolean] =
    runOnMVar(
      writeState.getOrElseUpdate(key, MVar(default)),
      StateT.modify[Task, V](modify).map(_ ⇒ true),
      readState.update(key, _: V)
    )

  protected def run[T](key: K, mod: StateT[Task, V, T], ifNotExists: V): Task[T] =
    runOnMVar(
      writeState.getOrElseUpdate(key, MVar(ifNotExists)),
      mod,
      readState.update(key, _: V)
    )
}

object MVarMapCache {
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
