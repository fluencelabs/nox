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

package fluence.effects.ipfs
import java.nio.ByteBuffer

import cats.Monad
import cats.data.EitherT
import com.softwaremill.sttp.{SttpBackend, Uri}
import fluence.effects.castore.{ContentAddressableStore, StoreError}
import fluence.log.Log
import scodec.bits.ByteVector

import scala.concurrent.duration._
import scala.language.higherKinds

/**
 * Implementation of IPFS downloading mechanism
 *
 * @param client to interact with IPFS nodes
 */
class IpfsStore[F[_]](client: IpfsClient[F]) extends ContentAddressableStore[F] {

  override def fetch(hash: ByteVector)(implicit log: Log[F]): EitherT[F, StoreError, fs2.Stream[F, ByteBuffer]] =
    client.download(hash)

  /**
   * Returns hash of files from directory.
   * If hash belongs to file, returns the same hash.
   *
   * @param hash Content's hash
   */
  override def ls(hash: ByteVector)(implicit log: Log[F]): EitherT[F, StoreError, List[ByteVector]] =
    client.ls(hash)
}

object IpfsStore {

  def apply[F[_]](
    address: Uri,
    readTimeout: FiniteDuration
  )(implicit F: Monad[F], sttpBackend: SttpBackend[EitherT[F, Throwable, ?], fs2.Stream[F, ByteBuffer]]): IpfsStore[F] =
    new IpfsStore(new IpfsClient[F](address, readTimeout))
}
