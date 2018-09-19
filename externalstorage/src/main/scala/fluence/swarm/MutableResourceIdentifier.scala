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

package fluence.swarm
import scodec.bits.ByteVector

import scala.concurrent.duration.FiniteDuration

/**
 * Parameters that describe the mutable resource and required for searching updates of the mutable resource.
 *
 * @param name optional resource name. You can use any name
 * @param frequency expected time interval between updates, in seconds
 * @param startTime time the resource is valid from, in Unix time (seconds). Set to the current epoch
 *                  You can also put a startTime in the past or in the future.
 *                  Setting it in the future will prevent nodes from finding content until the clock hits startTime.
 *                  Setting it in the past allows you to create a history for the resource retroactively
 * @param ownerAddr Swarm address (Ethereum wallet address)
 */
case class MutableResourceIdentifier(
  name: Option[String],
  frequency: FiniteDuration,
  startTime: FiniteDuration,
  ownerAddr: ByteVector
) {
  override def toString: String = {
    s"name: ${name.getOrElse("<null>")}, " +
      s"startTime: $startTime, " +
      s"frequency: $frequency, " +
      s"owner: 0x${ownerAddr.toHex}"
  }
}
