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

package fluence.effects.swarm

import java.nio.ByteBuffer

import cats.data.EitherT
import fluence.effects.castore.{ContentAddressableStore, StoreError}
import scodec.bits.ByteVector

import scala.language.higherKinds

class SwarmStore[F[_]](client: SwarmClient[F])(implicit F: cats.Monad[F]) extends ContentAddressableStore[F] {
  override def fetch(hash: ByteVector): EitherT[F, StoreError, fs2.Stream[F, ByteBuffer]] =
    client.fetch(hash.toHex).leftMap(identity[StoreError])

  override def ls(hash: ByteVector): EitherT[F, StoreError, List[ByteVector]] = EitherT.pure(List(hash))
}
