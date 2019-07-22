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
import java.nio.file.Paths

import cats.{Applicative, Monad}
import cats.data.EitherT
import fluence.effects.castore.{StorageToFileFailed, StoreError}
import fluence.log.Log
import scodec.bits.ByteVector
import fluence.crypto.hash.JdkCryptoHasher

import scala.language.higherKinds

/**
 * Mock of IpfsUploader, returns hashCode of an argument in a ByteVector
 */
class IpfsUploaderMock[F[_]: Applicative: Monad] extends IpfsUploader[F] {
  override def upload[A: IpfsData](data: A)(implicit log: Log[F]): EitherT[F, StoreError, ByteVector] =
    JdkCryptoHasher
      .Sha256[F](
        (data match {
          case bv: ByteVector        => bv
          case bvs: List[ByteVector] => bvs.fold(ByteVector.empty)((l, r) => l ++ r)
        }).toArray
      )
      .leftMap(e => StorageToFileFailed(ByteVector.empty, Paths.get("/"), e): StoreError)
      .map(ByteVector(_))
}
