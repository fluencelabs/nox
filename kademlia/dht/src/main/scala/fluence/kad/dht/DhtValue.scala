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

import cats.syntax.compose._
import cats.syntax.functor._
import fluence.codec.{CodecError, PureCodec}
import scodec.bits.ByteVector

import scala.util.Try

case class DhtValue[V](value: V, timestamp: Long)

object DhtValue {
  // Encode timestamp to and from a fixed-size byte array
  private val timestampCodec: PureCodec[Long, Array[Byte]] =
    PureCodec.build(
      PureCodec.liftFuncEither[Long, Array[Byte]](lng ⇒ Right(ByteVector.fromLong(lng).toArray)),
      PureCodec.liftFuncEither[Array[Byte], Long](
        bts ⇒
          Try(ByteVector(bts).toLong()).toEither.left.map(e ⇒ CodecError("Cannot decode Kademlia timestamp", Some(e)))
      )
    )

  // Take first 8 bytes and convert to long, or vice versa
  private val splitTimestampRest: PureCodec[(Long, Array[Byte]), Array[Byte]] =
    PureCodec.liftPointB(
      {
        case (lng, bs) ⇒
          timestampCodec.direct.pointAt(lng).map(Array.concat(_, bs))
      }, { bs ⇒
        val (lng, tail) = bs.splitAt(8)
        timestampCodec.inverse.pointAt(lng).map(_ -> tail)
      }
    )

  implicit def dhtValueCodec[V](implicit valueCodec: PureCodec[V, Array[Byte]]): PureCodec[DhtValue[V], Array[Byte]] =
    PureCodec.liftPointB[DhtValue[V], (Long, Array[Byte])](
      dv ⇒ valueCodec.direct.pointAt(dv.value).map(dv.timestamp -> _), {
        case (ts, vb) ⇒
          valueCodec.inverse.pointAt(vb).map(DhtValue(_, ts))
      }
    ) >>> splitTimestampRest
}
