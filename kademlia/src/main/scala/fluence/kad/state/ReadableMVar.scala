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

import cats.data.StateT
import cats.effect.concurrent.{MVar, Ref}
import cats.effect.syntax.bracket._
import cats.effect.{Async, Bracket, ExitCase}
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.~>

import scala.language.higherKinds

/**
 * ReadableMVar uses MVar as a write model (synchronized), and updates Ref read model when write model is changed,
 * so that reads are not deadlocked when state modification is in progress.
 *
 * @param mVar Write state
 * @param ref Read state
 * @param F Bracket to be used to recover write model in case state transition fails
 * @tparam F Effect
 * @tparam S State type
 */
private[state] class ReadableMVar[F[_], S] private (private val mVar: MVar[F, S], private val ref: Ref[F, S])(
  implicit F: Bracket[F, Throwable]
) {
  self ⇒

  /**
   * Apply the transition, using current state. No concurrent transitions possible.
   * During transition, read model is kept intact. Once transition complete, read model already holds the new state.
   *
   * @param mod State transition to perform
   * @tparam T Return value
   */
  def apply[T](mod: StateT[F, S, T]): F[T] =
    mVar.take.bracketCase { init ⇒
      // Run modification
      mod.run(init).flatMap {
        case (updated, value) ⇒
          for {
            _ ← ref.set(updated) // Read model is updated before write model is released to keep reads consistent
            _ ← mVar.put(updated)
          } yield value
      }
    } {
      // In case of error, revert initial value
      case (_, ExitCase.Completed) ⇒ F.unit
      case (init, _) ⇒ mVar.put(init)
    }

  /**
   * Non-blocking read
   */
  def read: F[S] = ref.get

  def run: StateT[F, S, ?] ~> F =
    λ[StateT[F, S, ?] ~> F](mod ⇒ self.apply(mod))
}

private[state] object ReadableMVar {

  /**
   * Provides an instance of ReadableMVar.
   *
   * @param init Initial valur
   * @tparam F Async effect
   * @tparam S State type
   */
  def of[F[_]: Async, S](init: S): F[ReadableMVar[F, S]] =
    for {
      mVar ← MVar.uncancelableOf[F, S](init)
      ref ← Ref.of[F, S](init)
    } yield new ReadableMVar[F, S](mVar, ref)
}
