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

package fluence.kad.testkit

import cats.data.StateT
import fluence.kad.Siblings
import fluence.kad.protocol.Key
import monix.eval.Coeval
import monix.execution.atomic.{Atomic, AtomicBoolean}

import scala.language.higherKinds

class TestSiblingOps[C](nodeId: Key, maxSiblingsSize: Int) extends Siblings.WriteOps[Coeval, C] {
  self ⇒

  private val state = Atomic(Siblings[C](nodeId, maxSiblingsSize))
  private val lock = AtomicBoolean(false)

  override protected def run[T](mod: StateT[Coeval, Siblings[C], T]): Coeval[T] = {
    Coeval {
      require(lock.compareAndSet(false, true), "Siblings must be unlocked")
      lock.set(true)
    }.flatMap(
      _ ⇒
        mod
          .run(read)
          .map {
            case (s, v) ⇒
              state.set(s)
              v
          }
          .doOnFinish(_ ⇒ Coeval.now(lock.compareAndSet(expect = true, update = false)))
    )
  }

  override def read: Siblings[C] =
    state.get

}
