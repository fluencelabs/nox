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
import cats.instances.list._
import cats.Traverse.ops._
import com.softwaremill.sttp.{asStream, sttp, SttpBackend, Uri}
import fluence.effects.Backoff
import fluence.effects.castore.{ContentAddressableStore, StoreError}
import fluence.effects.ipfs.ResponseOps._
import scodec.bits.ByteVector
import com.softwaremill.sttp.circe.asJson
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}

import scala.language.higherKinds

// TODO move somewhere else
object ResponseOps {
  import cats.ApplicativeError
  import cats.data.EitherT
  import cats.syntax.applicativeError._
  import com.softwaremill.sttp.Response

  implicit class RichResponse[F[_], T, EE <: Throwable](resp: F[Response[T]])(implicit F: ApplicativeError[F, EE]) {
    val toEitherT: EitherT[F, String, T] = resp.attemptT.leftMap(_.getMessage).subflatMap(_.body)
    def toEitherT[E](errFunc: String => E): EitherT[F, E, T] = toEitherT.leftMap(errFunc)
  }
}

case class FileManifest(Name: String, Hash: String, Size: Int, Type: Int)
case class IpfsObject(Hash: String, Links: List[FileManifest])
case class IpfsLs(Objects: List[IpfsObject])

object IpfsLs {
  implicit val encodeFileManifest: Encoder[FileManifest] = deriveEncoder
  implicit val decodeFileManifest: Decoder[FileManifest] = deriveDecoder
  implicit val encodeIpfsObject: Encoder[IpfsObject] = deriveEncoder
  implicit val decodeIpfsObject: Decoder[IpfsObject] = deriveDecoder
  implicit val encodeIpfsLs: Encoder[IpfsLs] = deriveEncoder
  implicit val decodeIpfsLs: Decoder[IpfsLs] = deriveDecoder
}

/**
 * Implementation of IPFS downloading mechanism
 *
 * @param ipfsUri URI of the IPFS node
 */
class IpfsStore[F[_]](ipfsUri: Uri)(
  implicit sttpBackend: SttpBackend[F, fs2.Stream[F, ByteBuffer]],
  F: cats.MonadError[F, StoreError],
  backoff: Backoff[IpfsError] = Backoff.default
) extends ContentAddressableStore[F] with slogging.LazyLogging {

  import IpfsLs._

  object Multihash {
    // https://github.com/multiformats/multicodec/blob/master/table.csv
    val SHA256 = ByteVector(0x12, 32) // 0x12 => SHA256; 32 = 256 bits in bytes
  }

  // URI for downloading the file
  private val CatUri = ipfsUri.path("/api/v0/cat")

  private val LsUri = ipfsUri.path("/api/v0/ls")

  // Converts 256-bits hash to an bas58 IPFS address, prepending multihash bytes
  private def toAddress(hash: ByteVector): String = (Multihash.SHA256 ++ hash).toBase58

  override def fetch(hash: ByteVector): EitherT[F, StoreError, fs2.Stream[F, ByteBuffer]] = {
    val address = toAddress(hash)
    val uri = CatUri.param("arg", address)
    logger.debug(s"IPFS download started $uri")
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

  private def lsRaw(hash: ByteVector): EitherT[F, StoreError, IpfsLs] = {
    val address = toAddress(hash)
    val uri = LsUri.param("arg", address)
    logger.debug(s"IPFS `ls` started $uri")
    sttp
      .response(asJson[IpfsLs])
      .get(uri)
      .send()
      .toEitherT { er =>
        val errorMessage = s"IPFS 'ls' error $uri: $er"
        IpfsError(errorMessage)
      }
      .subflatMap(_.left.map { er =>
        logger.error(s"Deserialization error: $er")
        IpfsError(s"IPFS 'ls' deserialization error $uri.", Some(er.error))
      })
      .map { r =>
        logger.debug(s"IPFS 'ls' finished $uri")
        r
      }
      .leftMap(identity[StoreError])
  }

  private def assert(test: Boolean, error: IpfsError): EitherT[F, StoreError, Unit] = {
    EitherT.fromEither(Either.cond(test, (), error.asInstanceOf[StoreError]))
  }

  /**
   * Returns hash of files from directory.
   * If hash belongs to file, returns the same hash.
   *
   * @param hash Content's hash
   */
  override def ls(hash: ByteVector): EitherT[F, StoreError, List[ByteVector]] =
    for {
      rawResponse <- lsRaw(hash)
      _ <- assert(
        rawResponse.Objects.size == 1,
        IpfsError(s"One Object should be in IPFS response. Response: $rawResponse")
      )
      rawHashes = {
        val headObject = rawResponse.Objects.head
        if (headObject.Links.isEmpty) List(headObject.Hash)
        else headObject.Links.map(_.Hash)
      }
      hashes <- rawHashes.map { h =>
        val a = EitherT
          .fromEither[F](ByteVector.fromHexDescriptive(h))
          .leftMap(err => IpfsError(s"Cannot parse '$h' hex: $err").asInstanceOf[StoreError])
        a
      }.sequence
    } yield {
      hashes
    }
}
