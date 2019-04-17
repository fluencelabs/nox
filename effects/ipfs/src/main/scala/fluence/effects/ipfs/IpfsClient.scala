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
import java.nio.file.{Files, Path}

import cats.{Monad, Show}
import cats.data.EitherT
import cats.instances.list._
import cats.Traverse.ops._
import com.softwaremill.sttp.Uri.QueryFragment.KeyValue
import com.softwaremill.sttp.{asStream, sttp, ByteArrayBody, Multipart, SttpBackend, Uri}
import com.softwaremill.sttp._
import fluence.effects.castore.StoreError
import scodec.bits.ByteVector
import com.softwaremill.sttp.circe.asJson
import io.circe.Decoder
import cats.instances.either._
import cats.syntax.either._

import scala.collection.immutable
import scala.language.higherKinds

// TODO move somewhere else
object ResponseOps {
  import cats.data.EitherT
  import com.softwaremill.sttp.Response

  implicit class RichResponse[F[_], T, EE <: Throwable](resp: EitherT[F, Throwable, Response[T]])(
    implicit F: Monad[F]
  ) {
    val toEitherT: EitherT[F, String, T] = resp.leftMap(_.getMessage).subflatMap(_.body)
    def toEitherT[E](errFunc: String => E): EitherT[F, E, T] = toEitherT.leftMap(errFunc)
  }
}

class IpfsClient[F[_]](ipfsUri: Uri)(
  implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], fs2.Stream[F, ByteBuffer]],
  F: cats.Monad[F]
) extends slogging.LazyLogging {

  import IpfsLsResponse._
  import ResponseOps._
  import IpfsClient._

  object Multihash {
    // https://github.com/multiformats/multicodec/blob/master/table.csv
    val SHA256 = ByteVector(0x12, 32) // 0x12 => SHA256; 32 = 256 bits in bytes
  }

  // URI for downloading the file
  private val CatUri = ipfsUri.path("/api/v0/cat")

  private val LsUri = ipfsUri.path("/api/v0/ls")

  private val UploadUri = ipfsUri.path("/api/v0/add")

  // Converts 256-bits hash to an bas58 IPFS address, prepending multihash bytes
  private def toAddress(hash: ByteVector): String = (Multihash.SHA256 ++ hash).toBase58

  private def fromAddress(str: String) = ByteVector.fromBase58Descriptive(str).map(_.drop(2))

  def download(hash: ByteVector): EitherT[F, StoreError, fs2.Stream[F, ByteBuffer]] = {
    val address = toAddress(hash)
    val uri = CatUri.param("arg", address)
    for {
      _ <- EitherT.pure[F, StoreError](logger.debug(s"IPFS download started $uri"))
      response <- sttp
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
    } yield response

  }

  private def lsRaw(hash: ByteVector): EitherT[F, StoreError, IpfsLsResponse] = {
    val address = toAddress(hash)
    val uri = LsUri.param("arg", address)
    for {
      _ <- EitherT.pure[F, StoreError](logger.debug(s"IPFS `ls` started $uri"))
      response <- sttp
        .response(asJson[IpfsLsResponse])
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
    } yield response

  }

  private def assert(test: Boolean, errorMessage: String): EitherT[F, StoreError, Unit] = {
    EitherT.fromEither(Either.cond(test, (), IpfsError(errorMessage): StoreError))
  }

  /**
   * Returns hash of files from directory.
   * If hash belongs to file, returns the same hash.
   *
   * @param hash Content's hash
   */
  def ls(hash: ByteVector): EitherT[F, StoreError, List[ByteVector]] =
    for {
      rawResponse <- lsRaw(hash)
      _ <- assert(
        rawResponse.Objects.size == 1,
        s"Expected a single object, got ${rawResponse.Objects.size}. Response: $rawResponse"
      )
      rawHashes = {
        val headObject = rawResponse.Objects.head
        if (headObject.Links.forall(_.Name.isEmpty)) List(headObject.Hash)
        else headObject.Links.map(_.Hash)
      }
      hashes <- rawHashes.map { h =>
        EitherT
          .fromEither[F](fromAddress(h))
          .leftMap(err => IpfsError(s"Cannot parse '$h' hex: $err"): StoreError)
      }.sequence
    } yield {
      logger.debug(s"IPFS 'ls' hashes: ${hashes.mkString(" ")}")
      hashes
    }

  /**
   * `add` operation. Wraps files with a directory if there are multiple files.
   *
   * @param multiparts parts to `add`
   * @param onlyHash if true calculates only hash - does not write to disk.
   * @return
   */
  private def add(
    multiparts: immutable.Seq[Multipart],
    onlyHash: Boolean
  ): EitherT[F, StoreError, ByteVector] = {

    // will wrap multiple files in a directory
    val multipleFlag = (multiparts.length > 1).toString

    val uri = UploadUri
      .queryFragment(KeyValue("pin", "true"))
      .queryFragment(KeyValue("path", ""))
      .queryFragment(KeyValue("only-hash", onlyHash.toString))
      .queryFragment(KeyValue("recursive", multipleFlag))
      .queryFragment(KeyValue("wrap-with-directory", multipleFlag))

    for {
      _ <- EitherT.pure[F, StoreError](logger.debug(s"IPFS upload started $uri"))
      // raw response: {upload-response-object}\n{upload-response-object}...
      responses <- sttp
        .response(asListJson[UploadResponse])
        .post(uri)
        .multipartBody(multiparts)
        .send()
        .toEitherT { er =>
          val errorMessage = s"IPFS 'add' error $uri: $er"
          IpfsError(errorMessage)
        }
        .subflatMap(_.left.map { er =>
          logger.error(s"Deserialization error: $er")
          IpfsError(s"IPFS 'add' deserialization error $uri.", Some(er.error))
        })
        .map { r =>
          logger.debug(s"IPFS 'add' finished $uri")
          r
        }
        .leftMap(identity[StoreError])
      _ <- assert(responses.nonEmpty, "IPFS 'add': Empty response.")
      namesWithHashes <- EitherT.fromEither[F](
        responses
          .map(
            r =>
              fromAddress(r.Hash).map(h => r.Name -> h).leftMap { e =>
                logger.debug(s"IPFS 'add' hash ${r.Hash} is not correct $uri")
                IpfsError(e)
            }
          )
          .sequence
      )
      hash <- {
        if (namesWithHashes.length == 1) EitherT.pure[F, StoreError](namesWithHashes.head._2)
        else {
          // if there is more then one JSON objects
          // find an object with an empty name - it will be an object with a directory
          EitherT.fromEither[F](
            namesWithHashes
              .find(_._1.isEmpty)
              .map(_._2)
              .toRight(
                IpfsError(s"IPFS 'add' incorrect response: $responses. No response with empty name."): StoreError
              )
          )
        }
      }
    } yield hash
  }

  /**
   * Only calculates hash - do not write to disk.
   *
   * @return hash of data
   */
  def calculateHash(data: ByteVector): EitherT[F, StoreError, ByteVector] =
    add(immutable.Seq(Multipart("", ByteArrayBody(data.toArray))), onlyHash = true)

  /**
   * Uploads bytes to IPFS node
   *
   * @return hash of data
   */
  def upload(data: ByteVector): EitherT[F, StoreError, ByteVector] =
    add(immutable.Seq(multipart("", data.toArray)), onlyHash = false)

  private def listPaths(path: Path): EitherT[F, StoreError, immutable.Seq[Path]] = {
    import scala.collection.JavaConverters._
    if (Files.isDirectory(path)) {
      val allFiles = Files.list(path).iterator().asScala.to[immutable.Seq]
      assert(allFiles.forall(p => Files.isRegularFile(p)), s"Directory ${path.getFileName} has nester directories.")
        .map(_ => allFiles)
    } else EitherT.pure(immutable.Seq(path))
  }

  /**
   * Uploads files to IPFS node. Supports only one file or files in one directory, without nested directories.
   *
   * @param path to a file or a directory
   * @return hash address
   */
  def upload(path: Path): EitherT[F, StoreError, ByteVector] = {
    for {
      _ <- assert(Files.exists(path), s"File '${path.getFileName}' does not exist")
      pathsList <- listPaths(path)
      parts = pathsList.map(p => multipartFile("", p))
      _ <- assert(parts.nonEmpty, s"Directory ${path.getFileName} is empty.")
      hash <- add(parts, onlyHash = false)
    } yield hash
  }
}

object IpfsClient {
  import io.circe.parser.decode

  // parses JSON like {object1}\n{object2}...
  def asListJson[B: Decoder: IsOption]: ResponseAs[Either[DeserializationError[io.circe.Error], List[B]], Nothing] = {
    asString.map(
      _.split("\\s+")
        .map(
          s =>
            decode[B](s).left
              .map(e => DeserializationError(s, e, Show[io.circe.Error].show(e)))
        )
        .toList
        .sequence
    )
  }
}
