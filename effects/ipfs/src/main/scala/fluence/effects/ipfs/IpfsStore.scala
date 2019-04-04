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

import cats.data.EitherT
import cats.effect.{Concurrent, ContextShift}
import cats.syntax.apply._
import com.softwaremill.sttp.{SttpBackend, sttp, _}
import fluence.effects.Backoff
import fluence.effects.castore.{ContentAddressableStore, StoreError}
import fluence.effects.ipfs.ResponseOps._
import scodec.bits.ByteVector

import scala.language.higherKinds

// TODO move somewhere else
object ResponseOps {
  import cats.ApplicativeError
  import cats.data.EitherT
  import cats.syntax.applicativeError._
  import com.softwaremill.sttp.Response

  implicit class RichResponse[F[_], T](resp: F[Response[T]])(implicit F: ApplicativeError[F, Throwable]) {
    val toEitherT: EitherT[F, String, T] = resp.attemptT.leftMap(_.getMessage).subflatMap(_.body)
    def toEitherT[E](errFunc: String => E): EitherT[F, E, T] = toEitherT.leftMap(errFunc)
  }
}

/**
 * Implementation of IPFS downloading mechanism
 *
 * @param ipfsUri URI of the IPFS node
 */
class IpfsStore[F[_]: Concurrent: ContextShift](ipfsUri: Uri)(
  implicit sttpBackend: SttpBackend[F, fs2.Stream[F, ByteBuffer]],
  backoff: Backoff[IpfsError] = Backoff.default
) extends ContentAddressableStore[F] with slogging.LazyLogging {

  object Multihash {
    // https://github.com/multiformats/multicodec/blob/master/table.csv
    val SHA256 = ByteVector(0x12, 32) // 0x12 => SHA256; 32 = 256 bits in bytes
  }

  // URI for downloading the file
  private val CatUri = ipfsUri.path("/api/v0/cat")

  private def getUri(addressBase58: String): Uri = CatUri.param("arg", addressBase58)

  // Converts 256-bits hash to an bas58 IPFS address, prepending multihash bytes
  private def toAddress(hash: ByteVector): String = (Multihash.SHA256 ++ hash).toBase58

  override def fetch(hash: ByteVector): EitherT[F, StoreError, fs2.Stream[F, ByteBuffer]] = {
    val address = toAddress(hash)
    val uri = getUri(address)

    EitherT.pure[F, StoreError](logger.debug(s"IPFS download started $uri")) *>
      sttp
        .response(asStream[fs2.Stream[F, ByteBuffer]])
        .get(uri)
        .send()
        .toEitherT { er =>
          val errorMessage = s"IPFS download error $uri: $er"
          IpfsError(errorMessage)
        }
        .map { r =>
          logger.debug(s"IPFS download finished $uri")
          r
        }
        .leftMap(identity[StoreError])
  }
}
