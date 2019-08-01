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

import cats.data.EitherT
import fluence.kad.protocol.Key
import fluence.log.Log
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * RPC interface for a single node's storage
 *
 * @tparam F Effect
 * @tparam V Value
 */
trait DhtRpc[F[_], V] {

  /**
   * Retrieve the value from node's local storage
   *
   */
  def retrieve(key: Key)(implicit log: Log[F]): EitherT[F, DhtError, V]

  /**
   * Kindly ask node to store the value in its local store.
   * Note that remote node may consider not to store the value or to modify it (e.g. combine with a semigroup).
   * You may need to check the value's consistency with a consequent [[retrieve]] call.
   *
   */
  def store(key: Key, value: V)(implicit log: Log[F]): EitherT[F, DhtError, Unit]

  /**
   * Retrieve hash of the value, if it is stored
   *
   */
  def retrieveHash(key: Key)(implicit log: Log[F]): EitherT[F, DhtError, ByteVector]
}
