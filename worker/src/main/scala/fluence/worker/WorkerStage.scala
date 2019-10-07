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

package fluence.worker

import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}

/**
 * Represents [[Worker]]'s lifecycle
 *
 * @param running Whether Worker is running or not
 */
sealed abstract class WorkerStage(val running: Boolean = false)

object WorkerStage {

  /**
   * Default stage for a freshly created [[WorkerContext]]
   */
  case object NotInitialized extends WorkerStage()

  /**
   * Asynchronous initialization of [[Worker]] has been started
   */
  case object InitializationStarted extends WorkerStage()

  /**
   * Worker is ready, all companions launched successfully
   */
  case object FullyAllocated extends WorkerStage(true)

  /**
   * [[WorkerContext.stop]] is triggered
   */
  case object Stopping extends WorkerStage()

  /**
   * Worker is stopped and is unable to handle requests
   * TODO can restart here
   */
  case object Stopped extends WorkerStage()

  /**
   * [[WorkerContext.destroy()]] is triggered: going to [[WorkerResource.destroy()]]
   */
  case object Destroying extends WorkerStage()

  /**
   * Worker is destroyed completely, there's no way to reuse it
   */
  case object Destroyed extends WorkerStage()

  implicit val encoder: Encoder[WorkerStage] = deriveEncoder
  implicit val decoder: Decoder[WorkerStage] = deriveDecoder
}
