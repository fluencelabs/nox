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

package fluence.kad.dht

import fluence.codec.{CodecError, PureCodec}
import scodec.bits.ByteVector

import scala.util.Try

/**
 * Metadata for the given DHT value.
 *
 * @param lastUpdated Last updated timetamp (seconds)
 */
case class DhtValueMetadata(lastUpdated: Long, hash: ByteVector)

object DhtValueMetadata {

  // TODO use some upgradeable data scheme
  implicit val dhtMetadataCodec: PureCodec[DhtValueMetadata, Array[Byte]] =
    PureCodec.build(
      PureCodec.liftFunc[DhtValueMetadata, Array[Byte]](
        dvm ⇒ (ByteVector.fromLong(dvm.lastUpdated) ++ dvm.hash).toArray
      ),
      PureCodec.liftFuncEither[Array[Byte], DhtValueMetadata](
        bytes ⇒
          Try {
            val (ts, rest) = bytes.splitAt(8)
            DhtValueMetadata(
              ByteVector(ts).toLong(),
              ByteVector(rest)
            )
          }.toEither.left.map(e ⇒ CodecError("Cannot decode DHT metadata", Some(e)))
      )
    )
}
