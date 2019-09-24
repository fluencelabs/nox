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

package fluence.statemachine.docker

import cats.Monad
import cats.effect.Resource
import fluence.effects.EffectError
import fluence.effects.docker.params.{DockerImage, DockerLimits, DockerParams}
import fluence.effects.docker.{DockerContainer, DockerIO, DockerNetwork}
import fluence.effects.sttp.SttpEffect
import fluence.log.Log
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.command.{PeersControl, ReceiptBus}
import fluence.statemachine.client.StateMachineClient
import shapeless._

import scala.language.higherKinds

/**
 * [[StateMachine]] launched inside a Docker container, with HTTP access to it
 */
object DockerStateMachine {
// TODO: in StateMachineHttp, it's taken from config
  val RpcPort: Short = 26662

  def make[F[_]: DockerIO: SttpEffect: Log: Monad](
    name: String,
    network: DockerNetwork,
    limits: DockerLimits,
    image: DockerImage,
    logLevel: Log.Level,
    environment: Map[String, String],
    vmCodePath: String,
    volumesFrom: Option[String],
    stopTimeout: Int
  ): Resource[F, StateMachine.Aux[F, DockerContainer :: ReceiptBus[F] :: PeersControl[F] :: HNil]] = {
    val internalMem = limits.memoryMb.map(mem => Math.floor(mem * 0.75).toInt)

    val params = DockerParams
      .build()
      .environment(environment)
      .option("-e", s"""CODE_DIR=$vmCodePath""")
      .option("-e", s"LOG_LEVEL=${logLevel.name}")
      .option("-e", internalMem.map(mem => s"WORKER_MEMORY_LIMIT=$mem"))
      .option("--name", name)
      .option("--network", network.name)
      .option("--volumes-from", volumesFrom.map(id => s"$id:ro"))
      .limits(limits)
      .prepared(image)
      .daemonRun()

    DockerIO[F]
      .run(params, stopTimeout)
      .map(
        c ⇒ StateMachineClient[F](name, RpcPort, DockerIO[F].checkContainer(c).leftMap(identity[EffectError])).extend(c)
      )
  }

}
