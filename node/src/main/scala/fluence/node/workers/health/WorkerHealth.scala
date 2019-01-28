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

package fluence.node.workers.health

import io.circe.generic.semiauto._
import io.circe.{Decoder, Encoder}

import scala.util.control.NoStackTrace

sealed trait WorkerHealth {
  def isHealthy: Boolean
}

object WorkerHealth {
  private implicit val encodeThrowable: Encoder[Throwable] = Encoder[String].contramap(_.getLocalizedMessage)

  private implicit val decodeThrowable: Decoder[Throwable] =
    Decoder[String].map(s => new Exception(s) with NoStackTrace)

  implicit val encoderWorkerInfo: Encoder[WorkerHealth] = deriveEncoder
  implicit val decoderWorkerInfo: Decoder[WorkerHealth] = deriveDecoder
}

sealed trait WorkerHealthy extends WorkerHealth {
  override def isHealthy: Boolean = true
}

sealed trait WorkerIll extends WorkerHealth {
  override def isHealthy: Boolean = false
}

case class WorkerRunning(uptime: Long, info: RunningWorkerInfo) extends WorkerHealthy

case class WorkerNotYetLaunched(info: StoppedWorkerInfo) extends WorkerIll

case class WorkerContainerNotRunning(info: StoppedWorkerInfo) extends WorkerIll

case class WorkerHttpCheckFailed(info: StoppedWorkerInfo, causedBy: Throwable) extends WorkerIll
