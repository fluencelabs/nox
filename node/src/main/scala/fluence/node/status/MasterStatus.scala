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

package fluence.node.status

import fluence.worker.WorkerStatus
import io.circe.generic.semiauto._
import io.circe.syntax._
import io.circe.{Decoder, Encoder, HCursor, Json}

import scala.language.postfixOps

/**
 * Master node status.
 *
 * @param ip master node external ip address
 * @param uptime working time of master node
 * @param numberOfWorkers number of registered workers
 * @param workers info about workers
 */
case class MasterStatus(
  ip: String,
  uptime: Long,
  numberOfWorkers: Int,
  workers: List[(Long, WorkerStatus)]
)

object MasterStatus {
  private implicit val encodeStatusTuple: Encoder[(Long, WorkerStatus)] = {
    case (appId: Long, status: WorkerStatus) ⇒
      Json.obj(
        ("appId", Json.fromLong(appId)),
        ("status", status.asJson)
      )
  }

  private implicit val decodeStatusTuple: Decoder[(Long, WorkerStatus)] = (c: HCursor) =>
    for {
      appId ← c.downField("appId").as[Long]
      status ← c.downField("status").as[WorkerStatus]
    } yield (appId, status)

  implicit val encodeMasterState: Encoder[MasterStatus] = deriveEncoder

  implicit val decodeMasterState: Decoder[MasterStatus] = deriveDecoder
}
