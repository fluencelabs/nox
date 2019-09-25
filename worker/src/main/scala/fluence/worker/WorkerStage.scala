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

sealed abstract class WorkerStage(val hasWorker: Boolean, val hasCompanions: Boolean)

object WorkerStage {

  case object NotInitialized extends WorkerStage(false, false)

  case object InitializationStarted extends WorkerStage(false, false)

  case object RunningCompanions extends WorkerStage(true, false)

  case object FullyAllocated extends WorkerStage(true, true)

  case object Stopping extends WorkerStage(false, false)

  // TODO can restart here
  case object Stopped extends WorkerStage(false, false)
  case object Destroying extends WorkerStage(false, false)
  case object Destroyed extends WorkerStage(false, false)

}
