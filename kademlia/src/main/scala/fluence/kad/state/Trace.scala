/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.kad.state

import cats.{Eval, Monoid, Traverse}
import cats.instances.list._

case class Trace private (trace: List[Eval[String]]) {
  def apply(log: ⇒ String): Trace = copy(Eval.later(log) :: trace)

  override def toString: String =
    s"Trace(${Traverse[List].sequence(trace).value.mkString(";\t|\t")})"
}

object Trace {
  def empty: Trace = new Trace(Nil)

  implicit object TraceMonoid extends Monoid[Trace] {
    override def empty: Trace = new Trace(Nil)

    override def combine(x: Trace, y: Trace): Trace =
      new Trace(x.trace ::: y.trace)
  }
}
