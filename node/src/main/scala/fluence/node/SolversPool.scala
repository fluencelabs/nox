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

package fluence.node

import cats.Parallel
import cats.effect.{Concurrent, ContextShift, Timer}
import cats.effect.concurrent.Ref
import cats.syntax.flatMap._
import cats.syntax.functor._
import com.softwaremill.sttp.SttpBackend
import fluence.node.Solver.HealthcheckConfig
import slogging.LazyLogging
import cats.instances.list._

import scala.language.higherKinds

class SolversPool[F[_]: Concurrent: ContextShift: Timer](
  solvers: Ref[F, Set[Solver[F]]],
  healthcheckConfig: HealthcheckConfig
)(
  implicit sttpBackend: SttpBackend[F, Nothing]
) extends LazyLogging {

  def run(params: Solver.Params): F[Unit] =
    for {
      solver <- Solver.run(params, healthcheckConfig)
      _ ← solvers.update(_ + solver)
      _ ← Concurrent[F].start(solver.fiber.join.flatMap { _ ⇒
        logger.debug(s"Removing solver from a pool: $solver")
        solvers.update(_ - solver)
      })
    } yield ()

  def stopAll[G[_]](implicit P: Parallel[F, G]): F[Unit] =
    for {
      ss ← solvers.get
      _ ← Parallel.parTraverse(ss.toList)(_.stop)
      _ ← solvers.set(Set.empty)
    } yield ()

}

object SolversPool {

  def apply[F[_]: Concurrent: ContextShift: Timer](
    implicit sttpBackend: SttpBackend[F, Nothing]
  ): F[SolversPool[F]] =
    for {
      solvers ← Ref.of[F, Set[Solver[F]]](Set.empty)
    } yield new SolversPool[F](solvers, HealthcheckConfig())
}
