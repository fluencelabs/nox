/*
 * Copyright (C) 2018  Fluence Labs Limited
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

package fluence.swarm.requests

import io.circe.Encoder
import io.circe.generic.semiauto.deriveEncoder
import scodec.bits.ByteVector

/**
 * Request for uploading a mutable resource's meta information.
 *
 * @param name optional resource name. You can use any name.
 * @param frequency expected time interval between updates, in seconds
 * @param startTime time the resource is valid from, in Unix time (seconds). Set to the current epoch
 *                  You can also put a startTime in the past or in the future.
 *                  Setting it in the future will prevent nodes from finding content until the clock hits startTime.
 *                  Setting it in the past allows you to create a history for the resource retroactively.
 * @param ownerAddr Swarm address (Ethereum wallet address)
 */
case class UploadMutableResourceRequest(name: Option[String], frequency: Long, startTime: Long, ownerAddr: ByteVector)

object UploadMutableResourceRequest {

  import fluence.swarm.ByteVectorCodec._

  implicit val uploadRequestEncoder: Encoder[UploadMutableResourceRequest] = deriveEncoder

}
